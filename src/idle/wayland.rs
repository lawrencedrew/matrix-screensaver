use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;
use super::{IdleDetector, IdleEvent};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notifier_v1::{self, ExtIdleNotifierV1},
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
};

pub struct WaylandIdleDetector;

#[async_trait]
impl IdleDetector for WaylandIdleDetector {
    async fn is_available(&self) -> bool {
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            return false;
        }
        // Try to connect and verify ext_idle_notifier_v1 is available
        tokio::task::spawn_blocking(|| {
            let Ok(conn) = Connection::connect_to_env() else { return false; };
            let Ok((globals, mut queue)) = registry_queue_init::<DummyState>(&conn) else { return false; };
            let qh = queue.handle();
            globals.bind::<ExtIdleNotifierV1, _, _>(&qh, 1..=1, ()).is_ok()
        })
        .await
        .unwrap_or(false)
    }

    async fn run(&self, timeout_secs: u64, tx: mpsc::Sender<IdleEvent>) -> Result<()> {
        let timeout_ms = (timeout_secs.saturating_mul(1000).min(u32::MAX as u64)) as u32;

        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = Connection::connect_to_env()?;
            let (globals, mut queue) = registry_queue_init::<AppState>(&conn)?;
            let qh = queue.handle();

            let notifier: ExtIdleNotifierV1 = globals.bind(&qh, 1..=1, ())?;
            let seat: WlSeat = globals.bind(&qh, 1..=8, ())?;
            let _notification = notifier.get_idle_notification(timeout_ms, &seat, &qh, ());

            let mut state = AppState { tx };
            loop {
                queue.blocking_dispatch(&mut state)?;
            }
        })
        .await?
    }
}

struct AppState {
    tx: mpsc::Sender<IdleEvent>,
}

struct DummyState;

impl Dispatch<WlRegistry, GlobalListContents> for DummyState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wayland_client::protocol::wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlRegistry, GlobalListContents> for AppState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wayland_client::protocol::wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<ExtIdleNotificationV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                let _ = state.tx.blocking_send(IdleEvent::Idle);
            }
            ext_idle_notification_v1::Event::Resumed => {
                let _ = state.tx.blocking_send(IdleEvent::Wake);
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotifierV1,
        _: ext_idle_notifier_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlSeat, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wayland_client::protocol::wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

wayland_client::delegate_noop!(DummyState: ignore WlSeat);
wayland_client::delegate_noop!(DummyState: ignore ExtIdleNotifierV1);
wayland_client::delegate_noop!(DummyState: ignore ExtIdleNotificationV1);
