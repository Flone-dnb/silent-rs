// External.
use iced::{
    button, text_input, Align, Button, Color, Column, Element, Length, Row, Text, TextInput,
};

// Custom.
use crate::global_params::*;
use crate::services::config_service::*;
use crate::services::net_service::ClientConfig;
use crate::services::user_tcp_service::{ConnectResult, IoResult};
use crate::themes::*;
use crate::MainMessage;

#[derive(Debug)]
pub struct ConnectLayout {
    pub username_string: String,
    pub servername_string: String,
    pub port_string: String,
    pub password_string: String,

    connect_result: ConnectResult,
    show_input_notice: bool,

    username_input: text_input::State,
    servername_input: text_input::State,
    port_input: text_input::State,
    password_input: text_input::State,
    connect_button: button::State,
    settings_button: button::State,
}

impl Default for ConnectLayout {
    fn default() -> Self {
        Self {
            port_string: DEFAULT_SERVER_PORT.to_string(),
            username_string: String::default(),
            servername_string: String::default(),
            password_string: String::default(),

            connect_result: ConnectResult::Ok,
            show_input_notice: false,

            username_input: text_input::State::default(),
            servername_input: text_input::State::default(),
            port_input: text_input::State::default(),
            password_input: text_input::State::default(),
            connect_button: button::State::default(),
            settings_button: button::State::default(),
        }
    }
}

impl ConnectLayout {
    pub fn read_user_config(&mut self) -> Result<(), String> {
        let config = UserConfig::new();
        if let Err(msg) = config {
            return Err(format!("{} at [{}, {}]", msg, file!(), line!()));
        }
        let config = config.unwrap();

        self.username_string = config.username;
        self.servername_string = config.server;
        self.port_string = config.server_port.to_string();
        self.password_string = config.server_password;

        Ok(())
    }
    pub fn save_user_config(&self) -> Result<(), String> {
        let config = UserConfig::new();
        if let Err(msg) = config {
            return Err(format!("{} at [{}, {}]", msg, file!(), line!()));
        }
        let mut config = config.unwrap();

        config.username = self.username_string.clone();
        config.server = self.servername_string.clone();
        config.server_port = self.port_string.parse::<u16>().unwrap();
        config.server_password = self.password_string.clone();

        config.save()
    }
    pub fn is_data_filled(&mut self) -> Result<ClientConfig, ()> {
        if self.servername_string.chars().count() > 1
            && self.username_string.chars().count() > 1
            && self.port_string.chars().count() > 1
        {
            Ok(ClientConfig {
                username: self.username_string.clone(),
                server_name: self.servername_string.clone(),
                server_port: self.port_string.clone(),
                server_password: self.password_string.clone(),
            })
        } else {
            self.show_input_notice = true;
            Err(())
        }
    }
    pub fn set_connect_result(&mut self, connect_result: ConnectResult) {
        self.connect_result = connect_result;
    }
    pub fn view(&mut self, current_style: &StyleTheme) -> Element<MainMessage> {
        let mut content = Column::new()
            .align_items(Align::Center)
            .push(Column::new().height(Length::FillPortion(10)))
            .push(
                Row::new()
                    .spacing(5)
                    .height(Length::FillPortion(30))
                    .push(Column::new().width(Length::FillPortion(30)))
                    .push(
                        Column::new()
                            .width(Length::FillPortion(15))
                            .spacing(10)
                            .padding(5)
                            .push(Text::new("Username: ").color(Color::WHITE))
                            .push(Text::new("Server: ").color(Color::WHITE))
                            .push(Text::new("Port: ").color(Color::WHITE))
                            .push(Text::new("Password: ").color(Color::WHITE)),
                    )
                    .push(
                        Column::new()
                            .width(Length::FillPortion(25))
                            .spacing(10)
                            .padding(5)
                            .push(
                                TextInput::new(
                                    &mut self.username_input,
                                    "Type your username...",
                                    &self.username_string,
                                    MainMessage::UsernameInputChanged,
                                )
                                .style(current_style.theme),
                            )
                            .push(
                                TextInput::new(
                                    &mut self.servername_input,
                                    "IP or domain name...",
                                    &self.servername_string,
                                    MainMessage::ServernameInputChanged,
                                )
                                .style(current_style.theme),
                            )
                            .push(
                                TextInput::new(
                                    &mut self.port_input,
                                    "",
                                    &self.port_string,
                                    MainMessage::PortInputChanged,
                                )
                                .style(current_style.theme),
                            )
                            .push(
                                TextInput::new(
                                    &mut self.password_input,
                                    "(optional)",
                                    &self.password_string,
                                    MainMessage::PasswordInputChanged,
                                )
                                .style(current_style.theme),
                            ),
                    )
                    .push(Column::new().width(Length::FillPortion(30))),
            )
            .push(Column::new().height(Length::FillPortion(5)))
            .push(
                Row::new()
                    .height(Length::Shrink)
                    .push(Column::new().width(Length::FillPortion(40)))
                    .push(
                        Button::new(
                            &mut self.connect_button,
                            Text::new("Connect").color(Color::WHITE),
                        )
                        .on_press(MainMessage::ConnectButtonPressed)
                        .width(Length::FillPortion(20))
                        .height(Length::Shrink)
                        .style(current_style.theme),
                    )
                    .push(Column::new().width(Length::FillPortion(40))),
            );

        if self.show_input_notice {
            content = content.push(
                Text::new("Please fill all non-optional fields.")
                    .color(Color::WHITE)
                    .height(Length::FillPortion(10)),
            )
        } else {
            content = content.push(Column::new().height(Length::FillPortion(10)));
        }

        let connect_text: String = match &self.connect_result {
            ConnectResult::Err(io_err) => match io_err {
                IoResult::FIN => {
                    String::from("An IO error occurred, the server closed connection.")
                }
                IoResult::Err(e) => format!("An IO error occurred, error: {}", e),
                _ => String::from("An IO error occurred."),
            },
            ConnectResult::OtherErr(msg) => format!("There was an error: {}", msg),
            _ => String::from(""),
        };

        content = content.push(Text::new(connect_text).color(Color::WHITE).size(25));

        content = content.push(Column::new().height(Length::FillPortion(10)));

        content = content
            .push(
                Row::new()
                    .height(Length::Shrink)
                    .push(Column::new().width(Length::FillPortion(40)))
                    .push(
                        Button::new(
                            &mut self.settings_button,
                            Text::new("Settings").color(Color::WHITE),
                        )
                        .on_press(MainMessage::ToSettingsButtonPressed)
                        .width(Length::FillPortion(20))
                        .height(Length::Shrink)
                        .style(current_style.theme),
                    )
                    .push(Column::new().width(Length::FillPortion(40))),
            )
            .push(Column::new().height(Length::FillPortion(10)));

        self.show_input_notice = false;

        content.into()
    }
}
