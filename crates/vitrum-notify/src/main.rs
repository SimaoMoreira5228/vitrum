use anyhow::Result;
use iced_layershell::settings::Settings;
use tokio::sync::mpsc;

mod app;
mod dbus;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();
	tracing::info!("Starting vitrum-notify daemon");

	let config = vitrum_config::Config::load().unwrap_or_default();
	let (tx, rx) = mpsc::unbounded_channel();

	let dbus_tx = tx.clone();
	let server = dbus::NotificationServer::new(dbus_tx);

	let _conn = zbus::connection::Builder::session()?
		.name("org.freedesktop.Notifications")?
		.serve_at("/org/freedesktop/Notifications", server)?
		.build()
		.await;

	let conn = match _conn {
		Ok(c) => c,
		Err(e) => {
			tracing::info!("Failed to acquire D-Bus name 'org.freedesktop.Notifications': {}. Is another notification daemon running? Exiting.", e);
			return Ok(());
		}
	};

	tracing::info!("D-Bus server initialized.");

	let ipc_tx = tx.clone();
	tokio::spawn(async move {
		let socket = vitrum_ipc::event_socket_path();
		if let Ok(mut subscriber) = vitrum_ipc::IpcEventSubscriber::connect(&socket).await {
			let _ = subscriber
				.send_request(
					vitrum_ipc::Opcode::Subscribe,
					&vitrum_ipc::protocol::IpcSubscribe {
						mask: vitrum_ipc::protocol::IpcEventMask::NOTIFICATION,
					},
				)
				.await;

			while let Ok((opcode, payload)) = subscriber.next_event().await {
				if let vitrum_ipc::Opcode::EventNotification = opcode {
					if let Ok(notif) = rmp_serde::from_slice::<vitrum_ipc::IpcEventNotification>(&payload) {
						let _ = ipc_tx.send(dbus::Notification {
							id: rand::random(),
							app_name: notif.app_name,
							summary: notif.summary,
							body: notif.body,
							expire_timeout: notif.timeout.unwrap_or(0) as i32,
						});
					}
				}
			}
		} else {
			tracing::warn!("Failed to connect to vitrum-ipc for notifications. Is the compositor running?");
		}
	});

	tracing::info!("IPC listener started. Starting UI...");

	let settings = Settings {
		id: Some("vitrum-notify".to_string()),
		layer_settings: app::layer_shell_settings(&config.notifications),
		..Default::default()
	};

	let rx_mutex = std::sync::Mutex::new(Some(rx));

	iced_layershell::application(
		move || app::boot(rx_mutex.lock().unwrap().take().expect("boot called twice"), config.clone()),
		"vitrum-notify",
		app::update,
		app::view,
	)
	.settings(settings)
	.subscription(app::subscription)
	.run()?;

	Ok(())
}
