use anyhow::Result;
use tracing::{info, warn};
use vitrum_config::KeyringConfig;
use zbus::blocking::Connection;

pub fn start(_config: &KeyringConfig) -> Result<Option<Vec<(String, String)>>> {
	let conn = match Connection::session() {
		Ok(c) => c,
		Err(e) => {
			warn!("Failed to connect to session bus for KeePassXC check: {}", e);
			return Ok(None);
		}
	};

	let has_owner: bool = match conn.call_method(
		Some("org.freedesktop.DBus"),
		"/org/freedesktop/DBus",
		Some("org.freedesktop.DBus"),
		"NameHasOwner",
		&("org.keepassxc.KeePassXC",),
	) {
		Ok(reply) => match reply.body().deserialize() {
			Ok(b) => b,
			Err(_) => return Ok(None),
		},
		Err(_) => return Ok(None),
	};

	if has_owner {
		info!("KeePassXC detected on D-Bus");
		Ok(Some(vec![]))
	} else {
		Ok(None)
	}
}
