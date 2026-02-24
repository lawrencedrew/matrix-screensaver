mod config;
mod idle;
mod render;

use tokio::sync::mpsc;
use idle::IdleEvent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = config::Config::load();
    apply_cli_args(&mut config);
    let backend = idle::detect_backend().await;

    let (tx, mut rx) = mpsc::channel::<IdleEvent>(8);

    let timeout = config.idle_timeout_secs;
    let tx_idle = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = backend.run(timeout, tx_idle).await {
            eprintln!("matrix-screensaver: idle backend error: {e}");
        }
    });

    let tx_lock = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = idle::lock::run_lock_listener(tx_lock).await {
            eprintln!("matrix-screensaver: lock listener error: {e}");
        }
    });

    drop(tx); // channel closes when both tasks end

    while let Some(event) = rx.recv().await {
        match event {
            IdleEvent::Idle => {
                let cfg = config.clone();
                let handle = tokio::task::spawn_blocking(move || {
                    render::run_screensaver(&cfg)
                });
                match handle.await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => eprintln!("matrix-screensaver: screensaver error: {e}"),
                    Err(e) => eprintln!("matrix-screensaver: screensaver panicked: {e}"),
                }
            }
            IdleEvent::Wake => {
                // SDL2 window exits on mouse/key activity — Wake events are informational
            }
        }
    }

    Ok(())
}

fn apply_cli_args(config: &mut config::Config) {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--timeout" => {
                if let Some(val) = args.get(i + 1) {
                    if let Ok(secs) = val.parse::<u64>() {
                        config.idle_timeout_secs = secs;
                    }
                    i += 1;
                }
            }
            "--color" => {
                if let Some(val) = args.get(i + 1) {
                    config.color = val.clone();
                    i += 1;
                }
            }
            "--fps" => {
                if let Some(val) = args.get(i + 1) {
                    if let Ok(fps) = val.parse::<u32>() {
                        config.fps = fps;
                    }
                    i += 1;
                }
            }
            "--speed" => {
                if let Some(val) = args.get(i + 1) {
                    if let Ok(speed) = val.parse::<f32>() {
                        config.speed = speed;
                    }
                    i += 1;
                }
            }
            "--charset" => {
                if let Some(val) = args.get(i + 1) {
                    if let Ok(cs) = val.parse::<config::Charset>() {
                        config.charset = cs;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
}
