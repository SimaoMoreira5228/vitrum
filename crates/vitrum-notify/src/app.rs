use crate::dbus::Notification;
use iced::widget::{Column, column, container, text};
use iced::{Element, Length, Subscription, Task, Theme};
use iced_layershell::actions::LayershellCustomActionWithId;
use iced_layershell::reexport::{Anchor, Layer};
use iced_layershell::settings::LayerShellSettings;
use std::time::Duration;
use tokio::sync::mpsc;
use vitrum_config::NotificationsConfig;

pub struct NotificationWrapper {
	pub inner: Notification,
	pub offset_x: f32,
}

pub struct NotifyState {
	notifications: Vec<NotificationWrapper>,
	receiver: mpsc::UnboundedReceiver<Notification>,
	config: vitrum_config::Config,
}

#[derive(Debug, Clone)]
pub enum Message {
	CheckNotifications,
	Tick,
	CloseNotification(u32),
}

impl TryFrom<Message> for LayershellCustomActionWithId {
	type Error = Message;
	fn try_from(msg: Message) -> Result<Self, Self::Error> {
		Err(msg)
	}
}

pub fn boot(receiver: mpsc::UnboundedReceiver<Notification>, config: vitrum_config::Config) -> (NotifyState, Task<Message>) {
	(
		NotifyState {
			notifications: Vec::new(),
			receiver,
			config,
		},
		Task::none(),
	)
}

pub fn update(state: &mut NotifyState, message: Message) -> Task<Message> {
	match message {
		Message::CheckNotifications => {
			while let Ok(mut n) = state.receiver.try_recv() {
				if n.expire_timeout <= 0 {
					n.expire_timeout = state.config.notifications.timeout as i32;
				}

				let offset_x = if state.config.notifications.slide {
					state.config.notifications.width as f32
				} else {
					0.0
				};

				state.notifications.push(NotificationWrapper { inner: n, offset_x });
			}
		}
		Message::Tick => {
			state.notifications.retain_mut(|wrapper| {
				let n = &mut wrapper.inner;

				if wrapper.offset_x > 0.0 {
					wrapper.offset_x = (wrapper.offset_x - 40.0).max(0.0);
				}

				if n.expire_timeout > 0 {
					n.expire_timeout -= 100;
					n.expire_timeout > 0
				} else {
					true
				}
			});
		}
		Message::CloseNotification(id) => {
			state.notifications.retain(|n| n.inner.id != id);
		}
	}
	Task::none()
}

pub fn view(state: &NotifyState) -> Element<'_, Message> {
	let mut col = Column::new().spacing(10);
	let p = Theme::Dark.palette();

	for wrapper in &state.notifications {
		let n = &wrapper.inner;
		let item = container(column![
			text(&n.app_name).size(12).color(p.primary),
			text(&n.summary).size(16).font(iced::Font {
				weight: iced::font::Weight::Bold,
				..Default::default()
			}),
			text(&n.body).size(14),
		])
		.padding(15)
		.width(Length::Fill)
		.style(move |_: &Theme| container::Style {
			background: Some(iced::Background::Color(p.background)),
			text_color: Some(p.text),
			border: iced::border::Border {
				color: p.primary,
				width: 1.0,
				radius: iced::border::Radius::from(8.0),
			},
			shadow: iced::Shadow {
				color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5),
				offset: iced::Vector::new(0.0, 2.0),
				blur_radius: 8.0,
			},
			..Default::default()
		});

		let row = if wrapper.offset_x > 0.0 {
			iced::widget::row![iced::widget::Space::new().width(Length::Fixed(wrapper.offset_x)), item]
		} else {
			iced::widget::row![item]
		};

		col = col.push(row);
	}

	container(col)
		.width(Length::Fixed(state.config.notifications.width as f32))
		.padding(state.config.notifications.margin as u16)
		.into()
}

pub fn subscription(_state: &NotifyState) -> Subscription<Message> {
	iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
}

pub fn layer_shell_settings(config: &NotificationsConfig) -> LayerShellSettings {
	let anchor = match config.position.as_str() {
		"top-left" => Anchor::Top | Anchor::Left,
		"top-center" => Anchor::Top,
		"bottom-left" => Anchor::Bottom | Anchor::Left,
		"bottom-right" => Anchor::Bottom | Anchor::Right,
		"bottom-center" => Anchor::Bottom,
		_ => Anchor::Top | Anchor::Right,
	};

	let m = config.margin as i32;

	LayerShellSettings {
		anchor,
		layer: Layer::Overlay,
		margin: (m, m, m, m),
		keyboard_interactivity: iced_layershell::reexport::KeyboardInteractivity::None,
		..Default::default()
	}
}
