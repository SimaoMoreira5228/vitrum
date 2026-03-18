use std::path::Path;

use anyhow::{Context, Result};
use tokio::net::UnixStream;

use crate::framing;
use crate::protocol::Opcode;

pub struct IpcClient {
	stream: UnixStream,
}

impl IpcClient {
	pub async fn connect(socket_path: &Path) -> Result<Self> {
		let mut stream = UnixStream::connect(socket_path)
			.await
			.context("Failed to connect to IPC socket")?;

		let (opcode, payload) = framing::receive_frame(&mut stream).await?;
		if opcode != Opcode::Handshake {
			anyhow::bail!("Expected Handshake, got {:?}", opcode);
		}
		let handshake: crate::protocol::IpcHandshake =
			rmp_serde::from_slice(&payload).context("Failed to deserialize handshake")?;
		tracing::debug!(compositor_version = %handshake.compositor_version, "Connected to Vitrum");

		Ok(Self { stream })
	}

	pub async fn send_request<Req: serde::Serialize>(&mut self, opcode: Opcode, payload: &Req) -> Result<(Opcode, Vec<u8>)> {
		let encoded = rmp_serde::to_vec_named(payload).context("Failed to serialize command")?;
		framing::send_frame(&mut self.stream, opcode, &encoded).await?;

		framing::receive_frame(&mut self.stream).await
	}

	pub fn into_stream(self) -> UnixStream {
		self.stream
	}
}

pub struct IpcEventSubscriber {
	stream: UnixStream,
}

impl IpcEventSubscriber {
	pub async fn connect(socket_path: &Path) -> Result<Self> {
		let mut stream = UnixStream::connect(socket_path)
			.await
			.context("Failed to connect to IPC event socket")?;

		let (opcode, payload) = framing::receive_frame(&mut stream).await?;
		if opcode != Opcode::Handshake {
			anyhow::bail!("Expected Handshake, got {:?}", opcode);
		}
		let handshake: crate::protocol::IpcHandshake =
			rmp_serde::from_slice(&payload).context("Failed to deserialize handshake")?;
		tracing::debug!(compositor_version = %handshake.compositor_version, "Connected to Vitrum Events");

		Ok(Self { stream })
	}

	pub async fn next_event(&mut self) -> Result<(Opcode, Vec<u8>)> {
		framing::receive_frame(&mut self.stream).await
	}

	pub async fn send_request<Req: serde::Serialize>(&mut self, opcode: Opcode, payload: &Req) -> Result<(Opcode, Vec<u8>)> {
		let encoded = rmp_serde::to_vec_named(payload).context("Failed to serialize request")?;
		framing::send_frame(&mut self.stream, opcode, &encoded).await?;

		framing::receive_frame(&mut self.stream).await
	}

	pub fn into_stream(self) -> UnixStream {
		self.stream
	}
}
