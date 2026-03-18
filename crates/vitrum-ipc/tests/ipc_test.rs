use anyhow::Result;
use tempfile::tempdir;
use vitrum_ipc::protocol::{IpcEventMask, IpcHandshake};
use vitrum_ipc::{IpcClient, IpcEventSubscriber, IpcServer, Opcode, WindowId, WindowInfo, WindowsResponse};

#[tokio::test]
async fn test_ipc_full_cycle() -> Result<()> {
	let dir = tempdir()?;
	let cmd_path = dir.path().join("cmd.sock");
	let evt_path = dir.path().join("evt.sock");

	let mut server = IpcServer::new(&cmd_path, &evt_path).await?;

	let server_handle = tokio::spawn(async move {
		let mut conn = server.accept_command().await.unwrap();

		let (opcode, _payload) = conn.receive_frame().await.unwrap();
		assert_eq!(opcode, Opcode::GetWindows);

		let response = WindowsResponse { windows: vec![] };
		conn.send_response(Opcode::ResponseWindows, &response).await.unwrap();

		let stream = server.accept_event_listener().await.unwrap();

		let mut evt_conn = vitrum_ipc::server::CommandConnection::new(stream);
		let (opcode, payload) = evt_conn.receive_frame().await.unwrap();
		assert_eq!(opcode, Opcode::Subscribe);
		let sub: vitrum_ipc::protocol::IpcSubscribe = rmp_serde::from_slice(&payload).unwrap();

		server.register_event_listener(evt_conn.into_stream(), sub.mask);

		let event = vitrum_ipc::protocol::events::WindowOpened {
			window: WindowInfo {
				id: WindowId(1),
				title: "Test".to_string(),
				app_id: "test".to_string(),
				workspace: 1,
				floating: false,
				fullscreen: false,
				opacity: 1.0,
				pinned: false,
				urgent: false,
			},
		};
		server
			.broadcast_event(IpcEventMask::WINDOW, Opcode::EventWindowOpened, &event)
			.await
			.unwrap();
	});

	let mut client = IpcClient::connect(&cmd_path).await?;

	let (opcode, payload) = client.send_request(Opcode::GetWindows, &()).await?;
	assert_eq!(opcode, Opcode::ResponseWindows);
	let resp: WindowsResponse = rmp_serde::from_slice(&payload)?;
	assert!(resp.windows.is_empty());

	let sub = IpcEventSubscriber::connect(&evt_path).await?;

	let sub_req = vitrum_ipc::protocol::IpcSubscribe {
		mask: IpcEventMask::WINDOW,
	};
	let _sub_encoded = rmp_serde::to_vec_named(&sub_req)?;

	let mut raw_sub = vitrum_ipc::server::CommandConnection::new(sub.into_stream());
	raw_sub.send_response(Opcode::Subscribe, &sub_req).await?;

	let (opcode, _payload) = raw_sub.receive_frame().await?;
	assert_eq!(opcode, Opcode::EventWindowOpened);

	server_handle.await?;
	Ok(())
}
