use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::mpsc;
use zbus::interface;

#[derive(Debug, Clone)]
pub struct Notification {
	pub id: u32,
	pub app_name: String,
	pub summary: String,
	pub body: String,
	pub expire_timeout: i32,
}

pub struct NotificationServer {
	sender: mpsc::UnboundedSender<Notification>,
	next_id: Arc<AtomicU32>,
}

impl NotificationServer {
	pub fn new(sender: mpsc::UnboundedSender<Notification>) -> Self {
		Self {
			sender,
			next_id: Arc::new(AtomicU32::new(1)),
		}
	}
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
	async fn notify(
		&mut self,
		app_name: &str,
		replaces_id: u32,
		_app_icon: &str,
		summary: &str,
		body: &str,
		_actions: Vec<&str>,
		_hints: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
		expire_timeout: i32,
	) -> u32 {
		let id = if replaces_id == 0 {
			self.next_id.fetch_add(1, Ordering::SeqCst)
		} else {
			replaces_id
		};

		let notification = Notification {
			id,
			app_name: app_name.to_string(),
			summary: summary.to_string(),
			body: body.to_string(),
			expire_timeout,
		};

		if let Err(e) = self.sender.send(notification) {
			tracing::error!("Failed to send notification to app: {}", e);
		}

		id
	}

	async fn close_notification(&mut self, id: u32) {
		tracing::info!("Requested close on notification {}", id);
	}

	async fn get_capabilities(&self) -> Vec<&str> {
		vec!["body"]
	}

	async fn get_server_information(&self) -> (String, String, String, String) {
		(
			"vitrum-notify".to_string(),
			"Vitrum".to_string(),
			"0.1.0".to_string(),
			"1.2".to_string(),
		)
	}
}
