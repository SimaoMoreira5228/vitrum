pub mod fixture;

use fixture::Fixture;

#[test]
fn test_fixture_creates_successfully() {
	let fixture = Fixture::new();
	assert_eq!(fixture.active_workspace(), 1);
}

#[test]
fn test_fixture_connects_client() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	assert!(fixture.has_client(client_id));
}

#[test]
fn test_fixture_window_visible_in_workspace() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	fixture.create_xdg_window(client_id, "test-app");
	assert_eq!(fixture.visible_window_count(), 1);
}

#[test]
fn test_fixture_multiple_windows() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	fixture.create_xdg_window(client_id, "app-1");
	fixture.create_xdg_window(client_id, "app-2");
	assert_eq!(fixture.visible_window_count(), 2);
}

#[test]
fn test_fixture_workspace_switching() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	fixture.create_xdg_window(client_id, "app-1");
	assert_eq!(fixture.active_workspace(), 1);

	fixture.switch_workspace(3);
	assert_eq!(fixture.active_workspace(), 3);
	assert_eq!(fixture.visible_window_count(), 0);

	fixture.switch_workspace(1);
	assert_eq!(fixture.visible_window_count(), 1);
}

#[test]
fn test_fixture_workspace_move_window() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	let win = fixture.create_xdg_window(client_id, "app-1").unwrap();
	assert_eq!(fixture.visible_window_count(), 1);

	fixture.move_window_to_workspace(win, 5);
	assert_eq!(fixture.visible_window_count(), 0);

	fixture.switch_workspace(5);
	assert_eq!(fixture.visible_window_count(), 1);
}

#[test]
fn test_fixture_workspace_next_prev() {
	let mut fixture = Fixture::new();
	assert_eq!(fixture.active_workspace(), 1);

	fixture.switch_workspace(5);
	assert_eq!(fixture.active_workspace(), 5);

	fixture.switch_workspace(10);
	assert_eq!(fixture.active_workspace(), 10);

	fixture.switch_workspace(1);
	assert_eq!(fixture.active_workspace(), 1);
}

#[test]
fn test_fixture_layout_dwindle() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	fixture.create_xdg_window(client_id, "app-1");
	fixture.create_xdg_window(client_id, "app-2");
	fixture.set_layout(vitrum_ipc::LayoutMode::Dwindle);
	assert_eq!(fixture.visible_window_count(), 2);
}

#[test]
fn test_fixture_layout_master_stack() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	fixture.create_xdg_window(client_id, "app-1");
	fixture.create_xdg_window(client_id, "app-2");
	fixture.set_layout(vitrum_ipc::LayoutMode::MasterStack);
	assert_eq!(fixture.visible_window_count(), 2);
}

#[test]
fn test_fixture_layout_floating() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	fixture.create_xdg_window(client_id, "app-1");
	fixture.create_xdg_window(client_id, "app-2");
	fixture.set_layout(vitrum_ipc::LayoutMode::Floating);
	assert_eq!(fixture.visible_window_count(), 2);
}

#[test]
fn test_fixture_many_windows() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	for i in 0..8 {
		fixture.create_xdg_window(client_id, &format!("app-{}", i));
	}
	assert_eq!(fixture.visible_window_count(), 8);
}

#[test]
fn test_fixture_workspace_isolation() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();

	fixture.create_xdg_window(client_id, "ws1-app");
	assert_eq!(fixture.visible_window_count(), 1);

	fixture.switch_workspace(2);
	fixture.create_xdg_window(client_id, "ws2-app");
	fixture.create_xdg_window(client_id, "ws2-app2");
	assert_eq!(fixture.visible_window_count(), 2);

	fixture.switch_workspace(1);
	assert_eq!(fixture.visible_window_count(), 1);
}

#[test]
fn test_fixture_remove_all_windows() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	let w1 = fixture.create_xdg_window(client_id, "app-1").unwrap();
	let w2 = fixture.create_xdg_window(client_id, "app-2").unwrap();
	let w3 = fixture.create_xdg_window(client_id, "app-3").unwrap();
	assert_eq!(fixture.visible_window_count(), 3);

	fixture.kill_workspace_window(w2);
	assert_eq!(fixture.visible_window_count(), 2);

	fixture.kill_workspace_window(w1);
	fixture.kill_workspace_window(w3);
	assert_eq!(fixture.visible_window_count(), 0);
}

#[test]
fn test_fixture_switch_to_invalid_workspace() {
	let mut fixture = Fixture::new();
	assert_eq!(fixture.active_workspace(), 1);

	fixture.switch_workspace(0);
	assert_eq!(fixture.active_workspace(), 1);

	fixture.switch_workspace(11);
	assert_eq!(fixture.active_workspace(), 1);
}

#[test]
fn test_fixture_move_to_same_workspace() {
	let mut fixture = Fixture::new();
	let client_id = fixture.add_client();
	let win = fixture.create_xdg_window(client_id, "app-1").unwrap();

	fixture.move_window_to_workspace(win, 1);
	assert_eq!(fixture.visible_window_count(), 1);
}
