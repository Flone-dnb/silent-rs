#![feature(linked_list_remove)]

// External.
use chrono::prelude::*;
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
use services::config_service::*;
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
    config.default_text_size = 26;
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

    ui_scaling: f64,
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
    MoveUserToRoom { username: String, room_to: String },
}

#[derive(Debug, Clone)]
pub enum MainMessage {
    MessageFromMainLayout(MainLayoutMessage),
    MessageFromSettingsLayout(SettingsLayoutMessage),
    MessageFromConnectLayout(ConnectLayoutMessage),
    MessageInputChanged(String),
    UsernameInputChanged(String),
    ServernameInputChanged(String),
    PortInputChanged(String),
    PasswordInputChanged(String),
    UIScalingSliderMoved(i32),
    Tick(()),
    ModalWindowMessage(ModalMessage),
    ToSettingsButtonPressed,
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
            ui_scaling: 1.0,
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

    fn scale_factor(&self) -> f64 {
        self.ui_scaling
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        // look for input
        let key_event = iced_native::subscription::events_with(|event, _status| match event {
            Event::Keyboard(keyboard_event) => match keyboard_event {
                keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::Tab,
                    modifiers,
                } => Some(MainMessage::MessageFromConnectLayout(
                    ConnectLayoutMessage::TabPressed,
                )),
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
                let mut guard_messages = self.internal_messages.lock().unwrap();
                for message in guard_messages.iter() {
                    match message {
                        InternalMessage::InitUserConfig => {
                            // Fill connect fields from config.
                            if let Err(msg) = self.connect_layout.read_user_config() {
                                self.connect_layout // use connect result to show this error
                                    .set_connect_result(ConnectResult::Err(format!(
                                        "{} at [{}, {}]",
                                        msg,
                                        file!(),
                                        line!()
                                    )));
                            }

                            // Apply settings.
                            let config = UserConfig::new();
                            if let Err(msg) = &config {
                                let error_msg = format!("{} at [{}, {}]", msg, file!(), line!());
                                self.connect_layout
                                    .set_connect_result(ConnectResult::Err(error_msg));
                            }
                            let config = config.unwrap();

                            self.settings_layout.ui_scaling_slider_value = config.ui_scaling as i32;
                            self.ui_scaling = config.ui_scaling as f64 / 100.0;
                        }
                        InternalMessage::SystemIOError(msg) => {
                            self.main_layout.add_system_message(msg.clone());
                        }
                        InternalMessage::MoveUserToRoom { username, room_to } => {
                            if let Err(msg) = self.main_layout.move_user(&username, &room_to) {
                                self.main_layout.add_system_message(msg);
                            } else {
                                if *username == self.main_layout.current_user_name {
                                    self.main_layout.clear_text_chat();
                                }
                            }
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
                            if let Err(msg) =
                                self.main_layout
                                    .add_user(username.clone(), String::from(""), false)
                            {
                                self.main_layout.add_system_message(format!(
                                    "{} at [{}, {}]",
                                    msg,
                                    file!(),
                                    line!()
                                ));
                            }
                        }
                        InternalMessage::UserDisconnected(username) => {
                            if let Err(msg) = self.main_layout.remove_user(&username) {
                                self.main_layout.add_system_message(msg);
                            }
                        }
                    }
                }
                guard_messages.clear();
            }
            MainMessage::ModalWindowMessage(message) => match message {
                ModalMessage::OkButtonPressed => self.main_layout.hide_modal_window(),
                ModalMessage::CloseModal => self.main_layout.hide_modal_window(),
            },
            MainMessage::MessageFromMainLayout(message) => match message {
                MainLayoutMessage::UserItemPressed(username) => {
                    self.main_layout.open_selected_user_info(username);
                }
                MainLayoutMessage::RoomItemPressed(room_name) => {
                    if self.main_layout.current_user_room != room_name {
                        if let Err(msg) = self.net_service.enter_room(&room_name) {
                            if msg.show_modal {
                                self.main_layout.show_modal_window(msg.message);
                            } else {
                                self.main_layout.add_system_message(msg.message);
                            }
                        } else {
                            self.main_layout.current_user_room = room_name;
                        }
                    }
                }
                MainLayoutMessage::HideUserInfoPressed => {
                    self.main_layout.hide_user_info();
                }
                MainLayoutMessage::MessageInputEnterPressed => {
                    if !self.main_layout.is_modal_window_showed() {
                        let message = self.main_layout.get_message_input();
                        if !message.is_empty() {
                            if let Err(msg) = self.net_service.send_user_message(message) {
                                if msg.show_modal {
                                    self.main_layout.show_modal_window(msg.message);
                                } else {
                                    self.main_layout.add_system_message(msg.message);
                                }
                            } else {
                                self.main_layout.clear_message_input();
                            }
                        }
                    }
                }
            },
            MainMessage::ToSettingsButtonPressed => {
                self.current_window_layout = WindowLayout::SettingsWindow
            }
            MainMessage::MessageInputChanged(text) => {
                if !self.main_layout.is_modal_window_showed() {
                    if text.chars().count() <= MAX_MESSAGE_SIZE {
                        self.main_layout.message_string = text
                    }
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
                if text.chars().count() <= MAX_PASSWORD_SIZE {
                    self.connect_layout.password_string = text;
                }
            }
            MainMessage::MessageFromConnectLayout(message) => match message {
                ConnectLayoutMessage::TabPressed => {
                    if self.current_window_layout == WindowLayout::ConnectWindow {
                        self.connect_layout.focus_on_next_item();
                    }
                }
                ConnectLayoutMessage::ConnectButtonPressed => {
                    if let Ok(config) = self.connect_layout.is_data_filled() {
                        let (tx, rx) = mpsc::channel();

                        self.net_service.start(
                            config,
                            self.connect_layout.username_string.clone(),
                            self.connect_layout.password_string.clone(),
                            tx,
                            Arc::clone(&self.internal_messages),
                        );

                        loop {
                            let received = rx.recv();
                            if received.is_err() {
                                // start() already finished probably because of wrong password wait
                                break;
                            }
                            let received = received.unwrap();

                            match received {
                                ConnectResult::Ok => {
                                    self.connect_layout.set_connect_result(ConnectResult::Ok);
                                    if let Err(msg) = self.main_layout.add_user(
                                        self.connect_layout.username_string.clone(),
                                        String::from(""),
                                        true,
                                    ) {
                                        self.main_layout.add_system_message(format!(
                                            "{} at [{}, {}]",
                                            msg,
                                            file!(),
                                            line!()
                                        ));
                                    }
                                    self.main_layout.current_user_name =
                                        self.connect_layout.username_string.clone();
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
                                ConnectResult::InfoAboutOtherUser(user_info, room) => {
                                    if let Err(msg) =
                                        self.main_layout.add_user(user_info.username, room, true)
                                    {
                                        self.main_layout.add_system_message(format!(
                                            "{} at [{}, {}]",
                                            msg,
                                            file!(),
                                            line!()
                                        ));
                                    }
                                }
                                ConnectResult::InfoAboutRoom(room_name) => {
                                    self.main_layout.add_room(room_name);
                                }
                                ConnectResult::SleepWithErr {
                                    message,
                                    sleep_in_sec,
                                } => {
                                    self.connect_layout
                                        .set_connect_result(ConnectResult::Err(message));

                                    self.net_service.password_retry = PasswordRetrySleep {
                                        sleep: true,
                                        sleep_time_sec: sleep_in_sec,
                                        sleep_time_start: Local::now(),
                                    }
                                }
                                _ => {
                                    self.connect_layout.set_connect_result(received);
                                    break;
                                }
                            }
                        }
                    }
                }
            },
            MainMessage::MessageFromSettingsLayout(message) => match message {
                SettingsLayoutMessage::GeneralSettingsButtonPressed => self
                    .settings_layout
                    .set_active_option(CurrentActiveOption::General),
                SettingsLayoutMessage::AboutSettingsButtonPressed => self
                    .settings_layout
                    .set_active_option(CurrentActiveOption::About),
                SettingsLayoutMessage::FromSettingsButtonPressed => {
                    if self.is_connected {
                        self.current_window_layout = WindowLayout::MainWindow
                    } else {
                        self.current_window_layout = WindowLayout::ConnectWindow
                    }
                }
                SettingsLayoutMessage::GithubButtonPressed => {
                    opener::open("https://github.com/Flone-dnb/silent-rs").unwrap();
                }
            },
            MainMessage::UIScalingSliderMoved(value) => {
                self.settings_layout.ui_scaling_slider_value = value;
                self.ui_scaling = value as f64 / 100.0;

                // Save to config.
                let config = UserConfig::new();
                if let Err(msg) = &config {
                    let error_msg = format!("{} at [{}, {}]", msg, file!(), line!());
                    if !self.is_connected {
                        self.connect_layout
                            .set_connect_result(ConnectResult::Err(error_msg));
                    } else {
                        self.main_layout.add_system_message(error_msg);
                    }
                }
                let mut config = config.unwrap();
                config.ui_scaling = value as u16;
                if let Err(msg) = config.save() {
                    let error_msg = format!("{} at [{}, {}]", msg, file!(), line!());
                    if !self.is_connected {
                        self.connect_layout
                            .set_connect_result(ConnectResult::Err(error_msg));
                    } else {
                        self.main_layout.add_system_message(error_msg);
                    }
                }
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
