use anyhow::{Result, bail};
use rustix::net::{RecvAncillaryBuffer, RecvFlags, SendAncillaryBuffer, SendAncillaryMessage, SendFlags, recvmsg, sendmsg};
use std::io::IoSlice;
use std::os::fd::{BorrowedFd, OwnedFd};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::{FRAME_HEADER_LEN, IPC_MAGIC, Opcode};

pub async fn send_frame<W: AsyncWriteExt + Unpin>(stream: &mut W, opcode: Opcode, payload: &[u8]) -> Result<()> {
	let len = payload.len() as u32;
	let mut header = [0u8; FRAME_HEADER_LEN];

	header[0..4].copy_from_slice(&IPC_MAGIC);
	header[4..6].copy_from_slice(&(opcode as u16).to_le_bytes());
	header[6..10].copy_from_slice(&len.to_le_bytes());

	stream.write_all(&header).await?;
	if len > 0 {
		stream.write_all(payload).await?;
	}

	Ok(())
}

pub async fn receive_frame<R: AsyncReadExt + Unpin>(stream: &mut R) -> Result<(Opcode, Vec<u8>)> {
	let mut header = [0u8; FRAME_HEADER_LEN];
	stream.read_exact(&mut header).await?;

	if &header[0..4] != &IPC_MAGIC {
		bail!("Invalid IPC magic bytes");
	}

	let opcode_val = u16::from_le_bytes(header[4..6].try_into().unwrap());
	let len = u32::from_le_bytes(header[6..10].try_into().unwrap()) as usize;

	if len > 1024 * 1024 * 10 {
		bail!("Frame too large: {} bytes", len);
	}

	let mut payload = vec![0u8; len];
	if len > 0 {
		stream.read_exact(&mut payload).await?;
	}

	Ok((Opcode::from(opcode_val), payload))
}

pub async fn send_fd(stream: &tokio::net::UnixStream, fd: BorrowedFd<'_>) -> Result<()> {
	loop {
		stream.writable().await?;

		match stream.try_io(tokio::io::Interest::WRITABLE, || {
			let mut anc_buf = [std::mem::MaybeUninit::<u8>::uninit(); 256];
			let mut ancillary = SendAncillaryBuffer::new(&mut anc_buf);
			let fds = [fd];
			ancillary.push(SendAncillaryMessage::ScmRights(&fds));

			let iov = [IoSlice::new(&[0u8; 1])];

			match sendmsg(stream, &iov, &mut ancillary, SendFlags::empty()) {
				Ok(_) => Ok(()),
				Err(e) => Err(std::io::Error::from_raw_os_error(e.raw_os_error())),
			}
		}) {
			Ok(_) => return Ok(()),
			Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
			Err(e) => return Err(e.into()),
		}
	}
}

pub async fn receive_fd(stream: &tokio::net::UnixStream) -> Result<OwnedFd> {
	loop {
		stream.readable().await?;

		match stream.try_io(tokio::io::Interest::READABLE, || {
			let mut anc_buf = [std::mem::MaybeUninit::<u8>::uninit(); 256];
			let mut ancillary = RecvAncillaryBuffer::new(&mut anc_buf);
			let mut buf = [0u8; 1];
			let mut iov = [std::io::IoSliceMut::new(&mut buf)];

			match recvmsg(stream, &mut iov, &mut ancillary, RecvFlags::empty()) {
				Ok(_) => {
					for msg in ancillary.drain() {
						if let rustix::net::RecvAncillaryMessage::ScmRights(fds) = msg {
							return Ok(fds.into_iter().next().unwrap());
						}
					}
					Err(std::io::Error::new(std::io::ErrorKind::Other, "No FD received"))
				}
				Err(e) => Err(std::io::Error::from_raw_os_error(e.raw_os_error())),
			}
		}) {
			Ok(fd) => return Ok(fd),
			Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
			Err(e) => return Err(e.into()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Cursor;

	#[tokio::test]
	async fn test_frame_roundtrip() -> Result<()> {
		let mut buffer = Cursor::new(Vec::new());
		let opcode = Opcode::GetWindows;
		let payload = b"hello vitrum";

		send_frame(&mut buffer, opcode, payload).await?;

		buffer.set_position(0);
		let (rec_opcode, rec_payload) = receive_frame(&mut buffer).await?;

		assert_eq!(rec_opcode, opcode);
		assert_eq!(rec_payload, payload);
		Ok(())
	}

	#[tokio::test]
	async fn test_invalid_magic() -> Result<()> {
		let mut buffer = Cursor::new(Vec::new());
		buffer.write_all(b"FAKE").await?;
		buffer.write_all(&[0u8; 6]).await?;

		buffer.set_position(0);
		let res = receive_frame(&mut buffer).await;
		assert!(res.is_err());
		assert!(res.err().unwrap().to_string().contains("Invalid IPC magic"));
		Ok(())
	}

	#[tokio::test]
	async fn test_empty_payload() -> Result<()> {
		let mut buffer = Cursor::new(Vec::new());
		send_frame(&mut buffer, Opcode::Handshake, &[]).await?;

		buffer.set_position(0);
		let (rec_opcode, rec_payload) = receive_frame(&mut buffer).await?;
		assert_eq!(rec_opcode, Opcode::Handshake);
		assert!(rec_payload.is_empty());
		Ok(())
	}
}
