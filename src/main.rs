use iced::{
    executor, window::icon::Icon, Application, Clipboard, Color, Command, Element, Settings,
};

use std::fs::File;
use std::sync::mpsc;

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
use services::user_net_service::ConnectResult;
use themes::StyleTheme;
use themes::Theme;

fn main() -> iced::Result {
    let mut config = Settings::default();
    config.antialiasing = false;
    config.window.size = (1100, 600);
    config.window.min_size = Some((1100, 600));
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

#[derive(Debug, Clone)]
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

    net_service: NetService,

    is_connected: bool,

    current_window_layout: WindowLayout,

    style: StyleTheme,
}

#[derive(Debug, Clone)]
pub enum MainMessage {
    MessageInputChanged(String),
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
}

impl Silent {
    fn new() -> Self {
        Silent {
            current_window_layout: WindowLayout::ConnectWindow,
            style: StyleTheme::new(Theme::Default),
            connect_layout: ConnectLayout::default(),
            settings_layout: SettingsLayout::default(),
            main_layout: MainLayout::default(),
            is_connected: false,
            net_service: NetService::new(),
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

    fn update(
        &mut self,
        message: Self::Message,
        clipboard: &mut Clipboard,
    ) -> Command<Self::Message> {
        match message {
            MainMessage::MessageInputChanged(text) => {
                if text.chars().count() <= MAX_MESSAGE_SIZE {
                    self.main_layout.message_string = text
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

                    self.net_service
                        .start(config, self.connect_layout.username_string.clone(), tx);

                    let received = rx.recv().unwrap();

                    if received == ConnectResult::Ok {
                        self.connect_layout.set_connect_result(ConnectResult::Ok);
                        self.current_window_layout = WindowLayout::MainWindow;
                    } else {
                        self.connect_layout.set_connect_result(received);
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
