use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;

pub mod wayland;
pub mod dbus;
pub mod x11;
pub mod lock;

#[derive(Debug, Clone, PartialEq)]
pub enum IdleEvent {
    Idle,
    Wake,
}

#[async_trait]
pub trait IdleDetector: Send + Sync {
    async fn is_available(&self) -> bool;
    async fn run(&self, timeout_secs: u64, tx: mpsc::Sender<IdleEvent>) -> Result<()>;
}

pub async fn detect_backend() -> Box<dyn IdleDetector> {
    let backends: Vec<Box<dyn IdleDetector>> = vec![
        Box::new(wayland::WaylandIdleDetector),
        Box::new(dbus::DbusIdleDetector),
        Box::new(x11::X11IdleDetector),
    ];
    for backend in backends {
        if backend.is_available().await {
            return backend;
        }
    }
    // Return a no-op backend that logs an error and idles forever
    // (avoids unwind panic in the tokio runtime)
    eprintln!("matrix-screensaver: no idle detection backend available. Ensure you are running X11 or a supported Wayland compositor.");
    std::process::exit(1);
}
