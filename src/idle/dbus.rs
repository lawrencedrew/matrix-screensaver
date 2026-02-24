use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;
use super::{IdleDetector, IdleEvent};

pub struct DbusIdleDetector;

#[async_trait]
impl IdleDetector for DbusIdleDetector {
    async fn is_available(&self) -> bool {
        let Ok(conn) = zbus::Connection::session().await else { return false; };
        // Check if either KDE or GNOME idle provider is available
        kde_idle_ms(&conn).await.is_ok() || gnome_idle_ms(&conn).await.is_ok()
    }

    async fn run(&self, timeout_secs: u64, tx: mpsc::Sender<IdleEvent>) -> Result<()> {
        let conn = zbus::Connection::session().await?;
        let timeout_ms = timeout_secs.saturating_mul(1000);
        let poll_interval = tokio::time::Duration::from_secs(5);
        let mut was_idle = false;

        loop {
            tokio::time::sleep(poll_interval).await;
            let idle_ms = match get_idle_ms(&conn).await {
                Ok(ms) => ms,
                Err(_) => continue,
            };

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

async fn get_idle_ms(conn: &zbus::Connection) -> Result<u64> {
    if let Ok(ms) = kde_idle_ms(conn).await {
        return Ok(ms);
    }
    if let Ok(ms) = gnome_idle_ms(conn).await {
        return Ok(ms);
    }
    anyhow::bail!("No idle time source found on D-Bus")
}

async fn kde_idle_ms(conn: &zbus::Connection) -> Result<u64> {
    let reply: u32 = conn
        .call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "GetSessionIdleTime",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    Ok(reply as u64)
}

async fn gnome_idle_ms(conn: &zbus::Connection) -> Result<u64> {
    let reply: u64 = conn
        .call_method(
            Some("org.gnome.Mutter.IdleMonitor"),
            "/org/gnome/Mutter/IdleMonitor/Core",
            Some("org.gnome.Mutter.IdleMonitor"),
            "GetIdletime",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    Ok(reply)
}
