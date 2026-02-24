use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;
use super::{IdleDetector, IdleEvent};

pub struct X11IdleDetector;

#[async_trait]
impl IdleDetector for X11IdleDetector {
    async fn is_available(&self) -> bool {
        std::env::var("DISPLAY").is_ok()
    }

    async fn run(&self, timeout_secs: u64, tx: mpsc::Sender<IdleEvent>) -> Result<()> {
        use x11rb::connection::Connection;
        use x11rb::protocol::screensaver;

        let (conn, screen_num) = x11rb::connect(None)?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let timeout_ms = timeout_secs * 1000;
        let poll = tokio::time::Duration::from_secs(5);
        let mut was_idle = false;

        loop {
            tokio::time::sleep(poll).await;
            let info = screensaver::query_info(&conn, root)?.reply()?;
            let idle_ms = info.ms_since_user_input as u64;

            if !was_idle && idle_ms >= timeout_ms {
                was_idle = true;
                let _ = tx.send(IdleEvent::Idle).await;
            } else if was_idle && idle_ms < timeout_ms {
                was_idle = false;
                let _ = tx.send(IdleEvent::Wake).await;
            }
        }
    }
}
