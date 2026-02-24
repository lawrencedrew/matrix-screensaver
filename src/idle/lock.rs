use tokio::sync::mpsc;
use anyhow::Result;
use futures_util::StreamExt;
use super::IdleEvent;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto",
)]
trait LoginSession {
    #[zbus(signal)]
    fn lock(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unlock(&self) -> zbus::Result<()>;
}

pub async fn run_lock_listener(tx: mpsc::Sender<IdleEvent>) -> Result<()> {
    let conn = zbus::Connection::system().await?;
    let proxy = LoginSessionProxy::new(&conn).await?;

    let mut lock_stream = proxy.receive_lock().await?;
    let mut unlock_stream = proxy.receive_unlock().await?;

    loop {
        tokio::select! {
            Some(_) = lock_stream.next() => {
                let _ = tx.send(IdleEvent::Idle).await;
            }
            Some(_) = unlock_stream.next() => {
                let _ = tx.send(IdleEvent::Wake).await;
            }
        }
    }
}
