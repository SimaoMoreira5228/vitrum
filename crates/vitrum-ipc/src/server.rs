use std::path::Path;

use anyhow::{Context, Result};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, info, warn};

use crate::framing;
use crate::protocol::{IpcEventMask, Opcode};

pub struct IpcServer {
	command_listener: UnixListener,
	event_listener: UnixListener,
	event_listeners: Vec<(UnixStream, IpcEventMask)>,
}

impl IpcServer {
	pub async fn new(command_path: &Path, event_path: &Path) -> Result<Self> {
		if let Some(parent) = command_path.parent() {
			tokio::fs::create_dir_all(parent)
				.await
				.context("Failed to create IPC socket directory")?;
		}

		if command_path.exists() {
			tokio::fs::remove_file(command_path)
				.await
				.context("Failed to remove old IPC command socket")?;
		}
		if event_path.exists() {
			tokio::fs::remove_file(event_path)
				.await
				.context("Failed to remove old IPC event socket")?;
		}

		let command_listener = UnixListener::bind(command_path).context("Failed to bind IPC command socket")?;
		let event_listener = UnixListener::bind(event_path).context("Failed to bind IPC event socket")?;

		info!(
			command = %command_path.display(),
			event = %event_path.display(),
			"IPC server started"
		);

		Ok(Self {
			command_listener,
			event_listener,
			event_listeners: Vec::new(),
		})
	}

	pub async fn accept_command(&mut self) -> Result<CommandConnection> {
		let (mut stream, addr) = self.command_listener.accept().await?;
		debug!(addr = ?addr, "Accepted IPC command connection");

		let handshake = crate::protocol::IpcHandshake {
			compositor_version: env!("CARGO_PKG_VERSION").to_string(),
			protocol_version: crate::protocol::IPC_VERSION,
			capabilities: crate::protocol::CAP_FD_PASSING,
		};
		let encoded = rmp_serde::to_vec_named(&handshake).context("Failed to serialize handshake")?;
		framing::send_frame(&mut stream, Opcode::Handshake, &encoded).await?;

		Ok(CommandConnection::new(stream))
	}

	pub async fn accept_event_listener(&mut self) -> Result<UnixStream> {
		let (mut stream, addr) = self.event_listener.accept().await?;
		debug!(addr = ?addr, "Accepted IPC event listener connection");

		let handshake = crate::protocol::IpcHandshake {
			compositor_version: env!("CARGO_PKG_VERSION").to_string(),
			protocol_version: crate::protocol::IPC_VERSION,
			capabilities: crate::protocol::CAP_FD_PASSING,
		};
		let encoded = rmp_serde::to_vec_named(&handshake).context("Failed to serialize handshake")?;
		framing::send_frame(&mut stream, Opcode::Handshake, &encoded).await?;

		Ok(stream)
	}

	pub fn register_event_listener(&mut self, stream: UnixStream, mask: IpcEventMask) {
		self.event_listeners.push((stream, mask));
	}

	pub async fn broadcast_event<T: serde::Serialize>(
		&mut self,
		mask: IpcEventMask,
		opcode: Opcode,
		event: &T,
	) -> Result<()> {
		if self.event_listeners.is_empty() {
			return Ok(());
		}

		let encoded = rmp_serde::to_vec_named(event).context("Failed to serialize event")?;
		let mut disconnected = Vec::new();

		for (idx, (listener, listener_mask)) in self.event_listeners.iter_mut().enumerate() {
			if listener_mask.contains(mask) {
				if let Err(e) = framing::send_frame(listener, opcode, &encoded).await {
					warn!(error = %e, "Event listener disconnected");
					disconnected.push(idx);
				}
			}
		}

		for idx in disconnected.into_iter().rev() {
			self.event_listeners.remove(idx);
		}

		Ok(())
	}

	pub fn event_listener_count(&self) -> usize {
		self.event_listeners.len()
	}
}

pub struct CommandConnection {
	stream: UnixStream,
}

impl CommandConnection {
	pub fn new(stream: UnixStream) -> Self {
		Self { stream }
	}

	pub fn into_stream(self) -> UnixStream {
		self.stream
	}

	pub async fn receive_frame(&mut self) -> Result<(Opcode, Vec<u8>)> {
		framing::receive_frame(&mut self.stream).await
	}

	pub async fn send_response<T: serde::Serialize>(&mut self, opcode: Opcode, response: &T) -> Result<()> {
		let encoded = rmp_serde::to_vec_named(response).context("Failed to serialize response")?;
		framing::send_frame(&mut self.stream, opcode, &encoded).await?;
		Ok(())
	}
}
