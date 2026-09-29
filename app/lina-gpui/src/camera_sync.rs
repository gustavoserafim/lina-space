//! `camera_sync` — a câmera (pan/zoom) de cada Espaço sobrevive a fechar/reabrir e à troca de Espaço.
//!
//! O core já tinha o store (`lina_core::SessionStore`, `camera.json` FORA do event log — ADR 0029 §3:
//! perder o arquivo perde só o enquadramento) mas nada o chamava: todo boot e toda troca de Espaço
//! voltava ao "home" e o usuário perdia onde estava trabalhando. Aqui mora a lógica gpui-free:
//! conversão com sanidade, e o "assentou?" que evita gravar a cada pixel de arrasto.

use std::path::{Path, PathBuf};

use lina_core::{session_dir, CameraSnapshot, SessionStore};

use crate::bridge::{Camera, ZOOM_MAX, ZOOM_MIN};

/// Câmera → snapshot de disco (f64).
#[must_use]
pub fn to_snapshot(cam: Camera) -> CameraSnapshot {
    CameraSnapshot {
        pan: (f64::from(cam.pan.0), f64::from(cam.pan.1)),
        zoom: f64::from(cam.zoom),
    }
}

/// Snapshot de disco → câmera. O disco pode ter sido editado à mão ou vir de outra versão: o zoom é
/// preso a `[ZOOM_MIN, ZOOM_MAX]` (a UI nunca desenha fora dos limites) e pan não-finito volta ao home.
#[must_use]
pub fn from_snapshot(snap: CameraSnapshot) -> Camera {
    let (px, py) = (snap.pan.0 as f32, snap.pan.1 as f32);
    if !px.is_finite() || !py.is_finite() || !snap.zoom.is_finite() {
        return Camera::default();
    }
    Camera {
        pan: (px, py),
        zoom: (snap.zoom as f32).clamp(ZOOM_MIN, ZOOM_MAX),
    }
}

/// Carrega a câmera do Espaço em `root` (ausente/corrompida ⇒ home). Nunca falha.
#[must_use]
pub fn load(root: &Path) -> Camera {
    SessionStore::open(session_dir(root)).map_or_else(
        |_| Camera::default(),
        |store| from_snapshot(store.load_camera()),
    )
}

/// Grava a câmera do Espaço em `root` (escrita atômica do store). Best-effort: falha só loga —
/// o pior caso é reabrir no enquadramento home.
fn save(root: &Path, cam: Camera) {
    let result = SessionStore::open(session_dir(root))
        .and_then(|store| store.save_camera(&to_snapshot(cam)));
    if let Err(error) = result {
        eprintln!(
            "lina-gpui: câmera não salva em {} ({error}) — reabre no enquadramento inicial",
            root.display()
        );
    }
}

/// Estado do "salvar quando assentar" de UMA câmera atrelada a um Espaço.
#[derive(Debug)]
pub struct CameraSync {
    root: PathBuf,
    /// O que está gravado em disco (ou o que foi carregado).
    saved: Camera,
    /// A câmera vista no tick anterior — igual à de agora = o usuário parou de mexer.
    seen: Camera,
}

impl CameraSync {
    /// Atrela ao Espaço `root` e devolve a câmera dele já carregada.
    #[must_use]
    pub fn open(root: PathBuf) -> (Self, Camera) {
        let cam = load(&root);
        (
            Self {
                root,
                saved: cam,
                seen: cam,
            },
            cam,
        )
    }

    /// Um tick periódico com a câmera de agora: grava quando ela ASSENTOU (mesma do tick anterior)
    /// e difere do que está em disco. Devolve se gravou.
    pub fn tick(&mut self, cam: Camera) -> bool {
        let settled = cam == self.seen;
        self.seen = cam;
        if settled && cam != self.saved {
            save(&self.root, cam);
            self.saved = cam;
            return true;
        }
        false
    }

    /// Grava já se a câmera difere do disco (ao trocar de Espaço/fechar) — sem esperar assentar.
    pub fn flush(&mut self, cam: Camera) {
        self.seen = cam;
        if cam != self.saved {
            save(&self.root, cam);
            self.saved = cam;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lina-camera-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn cam(px: f32, py: f32, zoom: f32) -> Camera {
        Camera {
            pan: (px, py),
            zoom,
        }
    }

    #[test]
    fn snapshot_roundtrip_and_insane_disk_values_are_tamed() {
        let c = cam(-120.5, 40.0, 1.5);
        assert_eq!(from_snapshot(to_snapshot(c)), c);
        let wild = CameraSnapshot {
            pan: (10.0, 20.0),
            zoom: 50.0,
        };
        assert_eq!(
            from_snapshot(wild).zoom,
            ZOOM_MAX,
            "zoom preso ao teto da UI"
        );
        let tiny = CameraSnapshot {
            pan: (0.0, 0.0),
            zoom: 0.001,
        };
        assert_eq!(from_snapshot(tiny).zoom, ZOOM_MIN);
        let nan = CameraSnapshot {
            pan: (f64::NAN, 0.0),
            zoom: 1.0,
        };
        assert_eq!(from_snapshot(nan), Camera::default(), "lixo volta ao home");
    }

    /// Só grava quando a câmera ASSENTA (mesma em dois ticks seguidos) e difere do disco; arrastar
    /// não grava a cada quadro; câmera igual à gravada não reescreve.
    #[test]
    fn saves_only_after_the_camera_settles_and_reloads_it() {
        let root = tmp("assenta");
        let (mut sync, start) = CameraSync::open(root.clone());
        assert_eq!(start, Camera::default(), "Espaço novo abre no home");

        assert!(!sync.tick(cam(-10.0, 0.0, 1.0)), "ainda mexendo (1º tick)");
        assert!(
            !sync.tick(cam(-50.0, 0.0, 1.0)),
            "ainda mexendo (mudou de novo)"
        );
        assert!(sync.tick(cam(-50.0, 0.0, 1.0)), "assentou → grava");
        assert!(
            !sync.tick(cam(-50.0, 0.0, 1.0)),
            "igual ao gravado → não reescreve"
        );
        assert_eq!(
            load(&root),
            cam(-50.0, 0.0, 1.0),
            "reabrir traz o enquadramento"
        );

        // Trocar de Espaço no meio de um arrasto: o flush não espera assentar.
        sync.flush(cam(5.0, 5.0, 0.8));
        assert_eq!(load(&root), cam(5.0, 5.0, 0.8));
        let (_, reopened) = CameraSync::open(root.clone());
        assert_eq!(reopened, cam(5.0, 5.0, 0.8));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Cada Espaço tem a sua câmera: uma raiz não enxerga a de outra.
    #[test]
    fn each_workspace_keeps_its_own_camera() {
        let (a_root, b_root) = (tmp("a"), tmp("b"));
        let (mut a, _) = CameraSync::open(a_root.clone());
        a.flush(cam(-1.0, -2.0, 1.2));
        assert_eq!(load(&b_root), Camera::default());
        let _ = std::fs::remove_dir_all(&a_root);
        let _ = std::fs::remove_dir_all(&b_root);
    }
}
