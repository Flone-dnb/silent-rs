// External.
use chrono::Local;
use druid::widget::prelude::*;
use druid::widget::{
    Button, CrossAxisAlignment, Flex, Label, LineBreaking, MainAxisAlignment, SizedBox, TextBox,
};
use druid::{Lens, LensExt, TextAlignment, WidgetExt};
use system_wide_key_state::*;

// Std.
use std::sync::{mpsc, Arc, Mutex};

// Custom.
use crate::global_params::*;
use crate::misc::formatter_max_characters::*;
use crate::services::audio_service::audio_service::UserVoiceData;
use crate::services::config_service::*;
use crate::services::net_service::*;
use crate::services::user_tcp_service::*;
use crate::ApplicationState;
use crate::Layout;

const WIDTH_SPACING: f64 = 2.0;

#[derive(Clone, Data, Lens)]
pub struct ConnectLayout {
    pub username: String,
    pub server: String,
    pub port: String,
    pub password: String,
    pub connect_result: String,
    pub show_input_notice: bool,
}

impl ConnectLayout {
    pub fn new() -> Self {
        ConnectLayout {
            username: String::new(),
            server: String::new(),
            port: String::from("51337"),
            password: String::new(),
            connect_result: String::new(),
            show_input_notice: false,
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .must_fill_main_axis(true)
            .with_flex_child(SizedBox::empty().expand(), 15.0)
            .with_flex_child(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .must_fill_main_axis(true)
                    .with_flex_child(SizedBox::empty().expand(), WIDTH_SPACING)
                    .with_flex_child(
                        Flex::column()
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .with_flex_child(
                                Label::new("Username: ").with_text_size(TEXT_SIZE).expand(),
                                1.0,
                            )
                            .with_default_spacer()
                            .with_flex_child(
                                Label::new("Server: ").with_text_size(TEXT_SIZE).expand(),
                                1.0,
                            )
                            .with_default_spacer()
                            .with_flex_child(
                                Label::new("Port: ").with_text_size(TEXT_SIZE).expand(),
                                1.0,
                            )
                            .with_default_spacer()
                            .with_flex_child(
                                Label::new("Password: ").with_text_size(TEXT_SIZE).expand(),
                                1.0,
                            ),
                        1.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), WIDTH_SPACING)
                    .with_flex_child(
                        Flex::column()
                            .with_flex_child(
                                TextBox::new()
                                    .with_placeholder("Type your username...")
                                    .with_text_size(TEXT_SIZE)
                                    .with_formatter(MaxCharactersFormatter::new(MAX_USERNAME_SIZE))
                                    .update_data_while_editing(true)
                                    .lens(
                                        ApplicationState::connect_layout
                                            .then(ConnectLayout::username),
                                    )
                                    .expand(),
                                1.0,
                            )
                            .with_default_spacer()
                            .with_flex_child(
                                TextBox::new()
                                    .with_placeholder("IP or domain name...")
                                    .with_text_size(TEXT_SIZE)
                                    .lens(
                                        ApplicationState::connect_layout
                                            .then(ConnectLayout::server),
                                    )
                                    .expand(),
                                1.0,
                            )
                            .with_default_spacer()
                            .with_flex_child(
                                TextBox::new()
                                    .with_text_size(TEXT_SIZE)
                                    .with_formatter(MaxCharactersFormatter::new(5))
                                    .update_data_while_editing(true)
                                    .lens(
                                        ApplicationState::connect_layout.then(ConnectLayout::port),
                                    )
                                    .expand(),
                                1.0,
                            )
                            .with_default_spacer()
                            .with_flex_child(
                                TextBox::new()
                                    .with_text_size(TEXT_SIZE)
                                    .with_placeholder("(optional)")
                                    .with_formatter(MaxCharactersFormatter::new(MAX_PASSWORD_SIZE))
                                    .update_data_while_editing(true)
                                    .lens(
                                        ApplicationState::connect_layout
                                            .then(ConnectLayout::password),
                                    )
                                    .expand(),
                                1.0,
                            ),
                        5.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), WIDTH_SPACING)
                    .expand(),
                35.0,
            )
            .with_default_spacer()
            .with_flex_child(
                Label::new(|data: &ApplicationState, _env: &_| {
                    if data.connect_layout.show_input_notice {
                        "Please fill all non-optional fields.".to_string()
                    } else {
                        data.connect_layout.connect_result.clone()
                    }
                })
                .with_text_size(TEXT_SIZE)
                .with_text_alignment(TextAlignment::Center)
                .with_line_break_mode(LineBreaking::WordWrap),
                5.0,
            )
            .with_flex_child(SizedBox::empty().expand(), 5.0)
            .with_flex_child(
                Flex::row()
                    .with_flex_child(SizedBox::empty().expand(), 35.0)
                    .with_flex_child(
                        Button::from_label(Label::new("Connect").with_text_size(TEXT_SIZE))
                            .on_click(ConnectLayout::on_connect_clicked)
                            .expand(),
                        30.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), 35.0),
                10.0,
            )
            .with_flex_child(SizedBox::empty().expand(), 10.0)
            .with_flex_child(
                Flex::row()
                    .with_flex_child(SizedBox::empty().expand(), 35.0)
                    .with_flex_child(
                        Button::from_label(Label::new("Settings").with_text_size(TEXT_SIZE))
                            .on_click(ConnectLayout::on_settings_clicked)
                            .expand(),
                        30.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), 35.0),
                10.0,
            )
            .with_flex_child(SizedBox::empty().expand(), 10.0)
    }
    pub fn read_user_config(&mut self, config: &UserConfig) -> Result<(), String> {
        self.username = config.username.clone();
        self.server = config.server.clone();
        self.port = config.server_port.to_string();
        self.password = config.server_password.clone();

        Ok(())
    }
    pub fn save_user_config(&self, data: &ApplicationState) -> Result<(), String> {
        let mut config_guard = data.user_config.lock().unwrap();

        config_guard.username = self.username.clone();
        config_guard.server = self.server.clone();
        config_guard.server_port = self.port.parse::<u16>().unwrap();
        config_guard.server_password = self.password.clone();

        config_guard.save()
    }
    pub fn is_data_filled(&mut self, push_to_talk_key: KeyCode) -> Result<ClientConfig, ()> {
        if self.server.chars().count() > 1
            && self.username.chars().count() > 1
            && self.port.chars().count() > 1
        {
            self.show_input_notice = false;
            Ok(ClientConfig {
                username: self.username.clone(),
                server_name: self.server.clone(),
                server_port: self.port.clone(),
                server_password: self.password.clone(),
                push_to_talk_key,
            })
        } else {
            self.show_input_notice = true;
            Err(())
        }
    }
    pub fn set_connect_result(&mut self, connect_result: ConnectResult) {
        self.connect_result = match connect_result {
            ConnectResult::IoErr(io_err) => match io_err {
                IoResult::FIN => {
                    String::from("An IO error occurred, the server closed connection.")
                }
                IoResult::Err(e) => format!("An IO error occurred, error: {}", e),
                _ => String::from("An IO error occurred."),
            },
            ConnectResult::Err(msg) => format!("There was an error: {}", msg),
            _ => String::from(""),
        };
    }
    fn check_fields_length(data: &mut ApplicationState) -> Result<(), String> {
        if data.connect_layout.username.chars().count() > MAX_USERNAME_SIZE {
            return Err(format!(
                "The username is too long ({} characters when the limit is {}.",
                data.connect_layout.username.chars().count(),
                MAX_USERNAME_SIZE
            ));
        }

        if data.connect_layout.password.chars().count() > MAX_PASSWORD_SIZE {
            return Err(format!(
                "The password is too long ({} characters when the limit is {}.",
                data.connect_layout.password.chars().count(),
                MAX_PASSWORD_SIZE
            ));
        }

        Ok(())
    }
    fn on_connect_clicked(ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.window_handle = Arc::new(Some(ctx.window().clone()));

        if let Err(msg) = ConnectLayout::check_fields_length(data) {
            data.connect_layout
                .set_connect_result(ConnectResult::Err(msg));
            return;
        }

        let config = data
            .connect_layout
            .is_data_filled(data.settings_layout.push_to_talk_keycode);
        if config.is_err() {
            return;
        }
        let config = config.unwrap();

        let (tx, rx) = mpsc::channel();
        let mut net_service_guard = data.network_service.lock().unwrap();

        net_service_guard.init_audio_service(Arc::clone(&data.audio_service));

        net_service_guard.start(
            config,
            data.connect_layout.username.clone(),
            data.connect_layout.password.clone(),
            tx,
            ctx.get_external_handle(),
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
                    data.connect_layout.set_connect_result(ConnectResult::Ok);
                    if let Err(msg) = data.main_layout.add_user(
                        data.connect_layout.username.clone(),
                        String::from(""),
                        0,
                        true,
                    ) {
                        data.main_layout.add_system_message(format!(
                            "{} at [{}, {}]",
                            msg,
                            file!(),
                            line!()
                        ));
                    }

                    data.main_layout.current_user_name = data.connect_layout.username.clone();
                    data.current_layout = Layout::Main;
                    data.is_connected = true;
                    data.main_layout.play_connect_sound();

                    // Save config.
                    if let Err(msg) = data.connect_layout.save_user_config(data) {
                        data.main_layout.add_system_message(format!(
                            "{} at [{}, {}]",
                            msg,
                            file!(),
                            line!()
                        ));
                    }
                    break;
                }
                ConnectResult::InfoAboutOtherUser(user_info, room, ping_ms) => {
                    {
                        let audio_guard = data.audio_service.lock().unwrap();

                        let mut users_audio_data_guard =
                            audio_guard.users_voice_data.lock().unwrap();

                        users_audio_data_guard.push(Arc::new(Mutex::new(UserVoiceData::new(
                            user_info.username.clone(),
                        ))));
                    }

                    if let Err(msg) =
                        data.main_layout
                            .add_user(user_info.username, room, ping_ms, true)
                    {
                        data.main_layout.add_system_message(format!(
                            "{} at [{}, {}]",
                            msg,
                            file!(),
                            line!()
                        ));
                    }
                }
                ConnectResult::InfoAboutRoom(room_name) => {
                    if data.main_layout.get_room_count() == 0 {
                        data.main_layout.current_user_room = room_name.clone();
                    }
                    data.main_layout.add_room(room_name);
                }
                ConnectResult::SleepWithErr {
                    message,
                    sleep_in_sec,
                } => {
                    data.connect_layout
                        .set_connect_result(ConnectResult::Err(message));

                    net_service_guard.password_retry = PasswordRetrySleep {
                        sleep: true,
                        sleep_time_sec: sleep_in_sec,
                        sleep_time_start: Local::now(),
                    }
                }
                _ => {
                    data.connect_layout.set_connect_result(received);
                    break;
                }
            }
        }
    }
    fn on_settings_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.current_layout = Layout::Settings;
    }
}
