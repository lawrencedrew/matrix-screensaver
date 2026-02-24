mod config;
mod idle;
mod render;

use tokio::sync::mpsc;
use idle::IdleEvent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::load();
    let backend = idle::detect_backend().await;

    let (tx, mut rx) = mpsc::channel::<IdleEvent>(8);

    let timeout = config.idle_timeout_secs;
    tokio::spawn(async move {
        if let Err(e) = backend.run(timeout, tx).await {
            eprintln!("matrix-screensaver: idle backend error: {e}");
        }
    });

    while let Some(event) = rx.recv().await {
        match event {
            IdleEvent::Idle => {
                let cfg = config.clone();
                let handle = tokio::task::spawn_blocking(move || {
                    render::run_screensaver(&cfg)
                });
                if let Err(e) = handle.await? {
                    eprintln!("matrix-screensaver: screensaver error: {e}");
                }
            }
            IdleEvent::Wake => {
                // SDL2 window exits on mouse/key activity — Wake events are informational
            }
        }
    }

    Ok(())
}
