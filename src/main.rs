use iced::widget::{button, center, container, text};
use iced::window;
use iced::{Element, Length, Point, Size, Subscription, Task, Theme, keyboard};
use std::collections::BTreeMap;

pub fn main() -> iced::Result {
    iced::daemon(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run()
}

struct App {
    windows: BTreeMap<window::Id, WindowType>,
    overlay_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowType {
    Main,
    Overlay,
}

#[derive(Debug, Clone)]
enum Message {
    OpenOverlay,
    WindowOpened(window::Id, WindowType),
    WindowClosed(window::Id),
    EventOccurred(iced::Event),
    OverlayMonitorSize(window::Id, Size),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let (_, open) = window::open(window::Settings {
            size: Size::new(400.0, 300.0),
            ..window::Settings::default()
        });

        (
            Self {
                windows: BTreeMap::new(),
                overlay_pending: false,
            },
            open.map(|id| Message::WindowOpened(id, WindowType::Main)),
        )
    }

    fn has_overlay(&self) -> bool {
        self.windows
            .values()
            .any(|t| matches!(t, WindowType::Overlay))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenOverlay => {
                if self.overlay_pending || self.has_overlay() {
                    return Task::none();
                }

                self.overlay_pending = true;

                let (_, open) = window::open(window::Settings {
                    // 不使用 fullscreen，避免全屏切换动画
                    decorations: false,
                    transparent: true,
                    resizable: false,
                    level: window::Level::AlwaysOnTop,
                    visible: true,

                    // 先给个初始值，后面在 WindowOpened 后再精确 resize
                    size: Size::new(800.0, 600.0),
                    position: window::Position::Specific(Point::new(0.0, 0.0)),

                    ..window::Settings::default()
                });

                open.map(|id| Message::WindowOpened(id, WindowType::Overlay))
            }

            Message::WindowOpened(id, window_type) => {
                self.windows.insert(id, window_type);

                if window_type == WindowType::Overlay {
                    self.overlay_pending = false;

                    // 创建后立刻移到左上角，再查询显示器大小并 resize
                    Task::batch(vec![
                        window::move_to(id, Point::new(0.0, 0.0)),
                        window::monitor_size(id)
                            .map(move |size| Message::OverlayMonitorSize(id, size.unwrap())),
                    ])
                } else {
                    Task::none()
                }
            }

            Message::OverlayMonitorSize(id, size) => Task::batch(vec![
                window::move_to(id, Point::new(0.0, 0.0)),
                window::resize(id, size),
            ]),

            Message::WindowClosed(id) => {
                self.windows.remove(&id);

                if self.windows.is_empty() {
                    iced::exit()
                } else {
                    Task::none()
                }
            }

            Message::EventOccurred(iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                ..
            })) => {
                if let keyboard::Key::Named(keyboard::key::Named::Escape) = key {
                    let tasks = self
                        .windows
                        .iter()
                        .filter_map(|(id, ty)| {
                            matches!(ty, WindowType::Overlay).then(|| window::close(*id))
                        })
                        .collect::<Vec<_>>();

                    Task::batch(tasks)
                } else {
                    Task::none()
                }
            }

            Message::EventOccurred(_) => Task::none(),
        }
    }

    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        match self.windows.get(&window_id) {
            Some(WindowType::Main) => {
                center(button("Create").on_press(Message::OpenOverlay)).into()
            }

            Some(WindowType::Overlay) => {
                container(center(text("Overlay Window (Press ESC to close)").size(30)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.55).into()),
                        ..Default::default()
                    })
                    .into()
            }

            None => iced::widget::space().into(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            window::close_events().map(Message::WindowClosed),
            iced::event::listen().map(Message::EventOccurred),
        ])
    }
}
