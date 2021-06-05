#![feature(linked_list_remove)]

// External.
use chrono::prelude::*;
use iced::{
    executor, keyboard, time, window::icon::Icon, Application, Clipboard, Color, Command, Element,
    Settings, Subscription,
};
use iced_native::Event;
use system_wide_key_state::*;

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
use services::audio_service::audio_service::*;
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

struct Silent {
    main_layout: MainLayout,
    connect_layout: ConnectLayout,
    settings_layout: SettingsLayout,
    current_window_layout: WindowLayout,

    internal_messages: Arc<Mutex<Vec<InternalMessage>>>,

    net_service: Arc<Mutex<NetService>>,
    audio_service: Arc<Mutex<AudioService>>,

    ui_scaling: f64,
    is_connected: bool,

    style: StyleTheme,
}

#[derive(Debug, Clone)]
pub enum InternalMessage {
    InitUserConfig,
    SystemIOError(String),
    UserMessage {
        username: String,
        message: String,
    },
    RefreshConnectedUsersCount(usize),
    ClearAllUsers,
    UserConnected(String),
    UserDisconnected(String),
    MoveUserToRoom {
        username: String,
        room_to: String,
    },
    UserPing {
        username: String,
        ping_ms: u16,
        try_again_number: u8, // when user was not found
    },
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
    MasterOutputVolumeSliderMoved(i32),
    Tick(()),
    ModalWindowMessage(ModalMessage),
    ToSettingsButtonPressed,
    ButtonPressed(keyboard::Event),
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
            net_service: Arc::new(Mutex::new(NetService::new())),
            audio_service: Arc::new(Mutex::new(AudioService::default())),
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
            Event::Keyboard(keyboard_event) => Some(MainMessage::ButtonPressed(keyboard_event)),
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
                let mut delayed_messages: Vec<InternalMessage> = Vec::new();
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
                            self.settings_layout.master_output_volume_slider_value =
                                config.master_volume as i32;
                            self.settings_layout.push_to_talk_key = config.push_to_talk_button;
                            self.ui_scaling = config.ui_scaling as f64 / 100.0;

                            self.audio_service
                                .lock()
                                .unwrap()
                                .init(Arc::clone(&self.net_service), config.master_volume as i32);
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
                        InternalMessage::UserPing {
                            username,
                            ping_ms,
                            try_again_number,
                        } => match self.main_layout.set_user_ping(username, *ping_ms) {
                            Ok(()) => {}
                            Err(()) => {
                                if *try_again_number == 0u8 {
                                    self.main_layout.add_system_message(format!(
                                        "Ping of user '{}' was received but no info about the user was received (ping of unknown user) [failed after {} attempts to wait for user info].",
                                        username,
                                        USER_CONNECT_FIRST_UDP_PING_RETRY_MAX_COUNT
                                    ));
                                } else {
                                    delayed_messages.push(InternalMessage::UserPing {
                                        username: String::from(username),
                                        ping_ms: *ping_ms,
                                        try_again_number: try_again_number - 1,
                                    });
                                }
                            }
                        },
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
                            if let Err(msg) = self.main_layout.add_user(
                                username.clone(),
                                String::from(""),
                                0,
                                false,
                            ) {
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
                guard_messages.append(&mut delayed_messages);
            }
            MainMessage::ButtonPressed(event) => match event {
                keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                } => {
                    // todo
                    if key_code == keyboard::KeyCode::Tab
                        && self.settings_layout.ask_for_push_to_talk_button == false
                    {
                        if self.current_window_layout == WindowLayout::ConnectWindow {
                            self.connect_layout.focus_on_next_item();
                        }
                    } else if self.settings_layout.ask_for_push_to_talk_button {
                        let mut key_code_internal = KeyCode::KG;
                        let mut not_set = false;
                        let mut skip = false;

                        if modifiers.shift {
                            key_code_internal = KeyCode::KShift;
                        } else if modifiers.control {
                            key_code_internal = KeyCode::KCtrl;
                        } else if modifiers.alt {
                            key_code_internal = KeyCode::KAlt;
                        } else {
                            match key_code {
                                keyboard::KeyCode::Escape => {
                                    skip = true;
                                }
                                keyboard::KeyCode::Tab => {
                                    key_code_internal = KeyCode::KTab;
                                }
                                keyboard::KeyCode::Q => {
                                    key_code_internal = KeyCode::KQ;
                                }
                                keyboard::KeyCode::W => {
                                    key_code_internal = KeyCode::KW;
                                }
                                keyboard::KeyCode::E => {
                                    key_code_internal = KeyCode::KE;
                                }
                                keyboard::KeyCode::R => {
                                    key_code_internal = KeyCode::KR;
                                }
                                keyboard::KeyCode::T => {
                                    key_code_internal = KeyCode::KT;
                                }
                                keyboard::KeyCode::Y => {
                                    key_code_internal = KeyCode::KY;
                                }
                                keyboard::KeyCode::U => {
                                    key_code_internal = KeyCode::KU;
                                }
                                keyboard::KeyCode::I => {
                                    key_code_internal = KeyCode::KI;
                                }
                                keyboard::KeyCode::O => {
                                    key_code_internal = KeyCode::KO;
                                }
                                keyboard::KeyCode::P => {
                                    key_code_internal = KeyCode::KP;
                                }
                                keyboard::KeyCode::A => {
                                    key_code_internal = KeyCode::KA;
                                }
                                keyboard::KeyCode::S => {
                                    key_code_internal = KeyCode::KS;
                                }
                                keyboard::KeyCode::D => {
                                    key_code_internal = KeyCode::KD;
                                }
                                keyboard::KeyCode::F => {
                                    key_code_internal = KeyCode::KF;
                                }
                                keyboard::KeyCode::G => {
                                    key_code_internal = KeyCode::KG;
                                }
                                keyboard::KeyCode::H => {
                                    key_code_internal = KeyCode::KH;
                                }
                                keyboard::KeyCode::J => {
                                    key_code_internal = KeyCode::KJ;
                                }
                                keyboard::KeyCode::K => {
                                    key_code_internal = KeyCode::KK;
                                }
                                keyboard::KeyCode::L => {
                                    key_code_internal = KeyCode::KL;
                                }
                                keyboard::KeyCode::Z => {
                                    key_code_internal = KeyCode::KZ;
                                }
                                keyboard::KeyCode::X => {
                                    key_code_internal = KeyCode::KX;
                                }
                                keyboard::KeyCode::C => {
                                    key_code_internal = KeyCode::KC;
                                }
                                keyboard::KeyCode::V => {
                                    key_code_internal = KeyCode::KV;
                                }
                                keyboard::KeyCode::B => {
                                    key_code_internal = KeyCode::KB;
                                }
                                keyboard::KeyCode::N => {
                                    key_code_internal = KeyCode::KN;
                                }
                                keyboard::KeyCode::M => {
                                    key_code_internal = KeyCode::KM;
                                }
                                _ => not_set = true,
                            }
                        }

                        if not_set == false {
                            if skip {
                                self.settings_layout.ask_for_push_to_talk_button = false;
                            } else {
                                self.settings_layout.push_to_talk_key = key_code_internal;
                                self.settings_layout.ask_for_push_to_talk_button = false;
                                self.settings_layout.push_to_talk_button_hint = "restart required";

                                // Save to config.
                                let config = UserConfig::new();
                                if let Err(msg) = &config {
                                    let error_msg =
                                        format!("{} at [{}, {}]", msg, file!(), line!());
                                    if !self.is_connected {
                                        self.connect_layout
                                            .set_connect_result(ConnectResult::Err(error_msg));
                                    } else {
                                        self.main_layout.add_system_message(error_msg);
                                    }
                                }
                                let mut config = config.unwrap();
                                config.push_to_talk_button = key_code_internal;
                                if let Err(msg) = config.save() {
                                    let error_msg =
                                        format!("{} at [{}, {}]", msg, file!(), line!());
                                    if !self.is_connected {
                                        self.connect_layout
                                            .set_connect_result(ConnectResult::Err(error_msg));
                                    } else {
                                        self.main_layout.add_system_message(error_msg);
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
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
                        if let Err(msg) = self.net_service.lock().unwrap().enter_room(&room_name) {
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
                            if let Err(msg) =
                                self.net_service.lock().unwrap().send_user_message(message)
                            {
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
                ConnectLayoutMessage::ConnectButtonPressed => {
                    if let Ok(config) = self
                        .connect_layout
                        .is_data_filled(self.settings_layout.push_to_talk_key)
                    {
                        let (tx, rx) = mpsc::channel();

                        let mut net_service_guard = self.net_service.lock().unwrap();

                        net_service_guard.init_audio_service(Arc::clone(&self.audio_service));

                        net_service_guard.start(
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
                                        0,
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
                                    self.main_layout.play_connect_sound();

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
                                ConnectResult::InfoAboutOtherUser(user_info, room, ping_ms) => {
                                    if let Err(msg) = self.main_layout.add_user(
                                        user_info.username,
                                        room,
                                        ping_ms,
                                        true,
                                    ) {
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

                                    net_service_guard.password_retry = PasswordRetrySleep {
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

                    self.settings_layout.ask_for_push_to_talk_button = false;
                    self.settings_layout.push_to_talk_button_hint = "";
                    self.settings_layout.master_volume_slider_hint = "";
                }
                SettingsLayoutMessage::PushToTalkChangeButtonPressed => {
                    self.settings_layout.ask_for_push_to_talk_button = true;
                }
                SettingsLayoutMessage::GithubButtonPressed => {
                    opener::open("https://github.com/Flone-dnb/silent-rs").unwrap();
                }
            },
            MainMessage::MasterOutputVolumeSliderMoved(value) => {
                self.settings_layout.master_output_volume_slider_value = value;
                self.settings_layout.master_volume_slider_hint = "restart required";

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
                config.master_volume = value as u16;
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
