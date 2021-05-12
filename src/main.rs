// External.
use iced::{
    executor, keyboard, time, window::icon::Icon, Application, Clipboard, Color, Command, Element,
    Settings, Subscription,
};
use iced_native::Event;

// Std.
use std::fs::File;
use std::sync::{mpsc, Arc, Mutex};

// Custom.
mod global_params;
mod layouts;
mod services;
mod themes;
mod widgets;
use global_params::*;
use layouts::connect_layout::*;
use layouts::main_layout::*;
use layouts::settings_layout::*;
use services::net_service::*;
use services::user_tcp_service::ConnectResult;
use themes::StyleTheme;
use themes::Theme;

fn main() -> iced::Result {
    let mut config = Settings::default();
    config.antialiasing = false;
    config.window.size = (1100, 600);
    config.window.min_size = Some((900, 600));
    config.default_font = Some(include_bytes!("../res/mplus-2p-light.ttf"));

    let icon = Icon::from_rgba(read_icon_png(String::from("res/app_icon.png")), 256, 256).unwrap();
    config.window.icon = Some(icon);
    config.default_text_size = 28;
    Silent::run(config)
}

fn read_icon_png(path: String) -> Vec<u8> {
    let decoder = png::Decoder::new(File::open(path).unwrap());
    let (info, mut reader) = decoder.read_info().unwrap();
    let mut buf = vec![0; info.buffer_size()];

    reader.next_frame(&mut buf).unwrap();

    buf
}

#[derive(Debug, Clone, PartialEq)]
enum WindowLayout {
    ConnectWindow,
    MainWindow,
    SettingsWindow,
}

#[derive(Debug)]
struct Silent {
    main_layout: MainLayout,
    connect_layout: ConnectLayout,
    settings_layout: SettingsLayout,

    internal_messages: Arc<Mutex<Vec<InternalMessage>>>,

    net_service: NetService,

    is_connected: bool,

    current_window_layout: WindowLayout,

    style: StyleTheme,
}

#[derive(Debug, Clone)]
pub enum InternalMessage {
    InitUserConfig,
    SystemIOError(String),
    UserMessage { username: String, message: String },
    RefreshConnectedUsersCount(usize),
    ClearAllUsers,
    UserConnected(String),
    UserDisconnected(String),
}

#[derive(Debug, Clone)]
pub enum MainMessage {
    MessageInputChanged(String),
    MessageInputEnterPressed,
    UsernameInputChanged(String),
    ServernameInputChanged(String),
    PortInputChanged(String),
    PasswordInputChanged(String),
    ConnectButtonPressed,
    ToSettingsButtonPressed,
    GeneralSettingsButtonPressed,
    AboutSettingsButtonPressed,
    GithubButtonPressed,
    FromSettingsButtonPressed,
    TabPressed,
    Tick(()),
}

impl Silent {
    fn new() -> Self {
        let messages = vec![InternalMessage::InitUserConfig];

        Silent {
            current_window_layout: WindowLayout::ConnectWindow,
            style: StyleTheme::new(Theme::Default),
            connect_layout: ConnectLayout::default(),
            settings_layout: SettingsLayout::default(),
            main_layout: MainLayout::default(),
            is_connected: false,
            net_service: NetService::new(),
            internal_messages: Arc::new(Mutex::new(messages)),
        }
    }
}

impl Application for Silent {
    type Executor = executor::Default;
    type Message = MainMessage;
    type Flags = ();

    fn new(_flags: ()) -> (Silent, Command<MainMessage>) {
        (Silent::new(), Command::none())
    }

    fn background_color(&self) -> Color {
        self.style.get_background_color()
    }

    fn title(&self) -> String {
        String::from("Silent")
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        // look for input
        let key_event = iced_native::subscription::events_with(|event, _status| match event {
            Event::Keyboard(keyboard_event) => match keyboard_event {
                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::Tab,
                    modifiers,
                } => Some(MainMessage::TabPressed),
                _ => None,
            },
            _ => None,
        });

        // look for new internal messages one time per second
        let tick_event = time::every(std::time::Duration::from_millis(
            INTERVAL_INTERNAL_MESSAGE_MS,
        ))
        .map(|_| MainMessage::Tick(()));

        Subscription::batch(vec![key_event, tick_event])
    }

    fn update(
        &mut self,
        message: Self::Message,
        clipboard: &mut Clipboard,
    ) -> Command<Self::Message> {
        match message {
            MainMessage::Tick(_) => {
                let mut guard = self.internal_messages.lock().unwrap();
                for message in guard.iter() {
                    match message {
                        InternalMessage::InitUserConfig => {
                            if let Err(msg) = self.connect_layout.read_user_config() {
                                self.connect_layout // use connect result to show this error
                                    .set_connect_result(ConnectResult::OtherErr(format!(
                                        "{} at [{}, {}]",
                                        msg,
                                        file!(),
                                        line!()
                                    )));
                            }
                        }
                        InternalMessage::SystemIOError(msg) => {
                            self.main_layout.add_system_message(msg.clone());
                        }
                        InternalMessage::UserMessage { username, message } => {
                            self.main_layout
                                .add_message(message.clone(), username.clone());
                        }
                        InternalMessage::RefreshConnectedUsersCount(count) => {
                            self.main_layout.connected_users = *count;
                        }
                        InternalMessage::ClearAllUsers => {
                            self.main_layout.clear_all_users();
                        }
                        InternalMessage::UserConnected(username) => {
                            self.main_layout.add_user(username.clone(), false);
                        }
                        InternalMessage::UserDisconnected(username) => {
                            if let Err(msg) = self.main_layout.remove_user(username.clone()) {
                                self.main_layout.add_system_message(msg);
                            }
                        }
                    }
                }
                guard.clear();
            }
            MainMessage::TabPressed => {
                if self.current_window_layout == WindowLayout::ConnectWindow {
                    self.connect_layout.focus_on_next_item();
                }
            }
            MainMessage::MessageInputChanged(text) => {
                if text.chars().count() <= MAX_MESSAGE_SIZE {
                    self.main_layout.message_string = text
                }
            }
            MainMessage::MessageInputEnterPressed => {
                let message = self.main_layout.get_message_input();
                if !message.is_empty() {
                    if let Err(msg) = self.net_service.send_user_message(message) {
                        self.main_layout.add_system_message(msg);
                    }
                    self.main_layout.clear_message_input();
                }
            }
            MainMessage::UsernameInputChanged(text) => {
                if text.chars().count() <= MAX_USERNAME_SIZE {
                    self.connect_layout.username_string = text;
                }
            }
            MainMessage::ServernameInputChanged(text) => {
                self.connect_layout.servername_string = text;
            }
            MainMessage::PortInputChanged(text) => {
                if text.chars().count() <= 5 {
                    self.connect_layout.port_string = text;
                }
            }
            MainMessage::PasswordInputChanged(text) => {
                self.connect_layout.password_string = text;
            }
            MainMessage::ConnectButtonPressed => {
                if let Ok(config) = self.connect_layout.is_data_filled() {
                    let (tx, rx) = mpsc::channel();

                    self.net_service.start(
                        config,
                        self.connect_layout.username_string.clone(),
                        tx,
                        Arc::clone(&self.internal_messages),
                    );

                    loop {
                        let received = rx.recv().unwrap();

                        match received {
                            ConnectResult::Ok => {
                                self.connect_layout.set_connect_result(ConnectResult::Ok);
                                self.main_layout
                                    .add_user(self.connect_layout.username_string.clone(), true);
                                self.current_window_layout = WindowLayout::MainWindow;
                                self.is_connected = true;

                                // Save config.
                                if let Err(msg) = self.connect_layout.save_user_config() {
                                    self.main_layout.add_system_message(format!(
                                        "{} at [{}, {}]",
                                        msg,
                                        file!(),
                                        line!()
                                    ));
                                }
                                break;
                            }
                            ConnectResult::InfoAboutOtherUser(user_info) => {
                                self.main_layout.add_user(user_info.username, true);
                            }
                            _ => {
                                self.connect_layout.set_connect_result(received);
                                break;
                            }
                        }
                    }
                }
            }
            MainMessage::ToSettingsButtonPressed => {
                self.current_window_layout = WindowLayout::SettingsWindow
            }
            MainMessage::GeneralSettingsButtonPressed => self
                .settings_layout
                .set_active_option(CurrentActiveOption::General),
            MainMessage::AboutSettingsButtonPressed => self
                .settings_layout
                .set_active_option(CurrentActiveOption::About),
            MainMessage::FromSettingsButtonPressed => {
                if self.is_connected {
                    self.current_window_layout = WindowLayout::MainWindow
                } else {
                    self.current_window_layout = WindowLayout::ConnectWindow
                }
            }
            MainMessage::GithubButtonPressed => {
                opener::open("https://github.com/Flone-dnb/silent-rs").unwrap();
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<MainMessage> {
        match self.current_window_layout {
            WindowLayout::ConnectWindow => self.connect_layout.view(&self.style),
            WindowLayout::SettingsWindow => self.settings_layout.view(&self.style),
            WindowLayout::MainWindow => self.main_layout.view(&self.style),
        }
    }
}
