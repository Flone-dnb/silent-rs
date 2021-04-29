use iced::{
    button, text_input, Align, Button, Color, Column, Element, Length, Row, Text, TextInput,
};

use crate::themes::*;
use crate::MainMessage;

#[derive(Debug, Default)]
pub struct ConnectLayout {
    pub username_string: String,
    pub servername_string: String,
    pub port_string: String,
    pub password_string: String,

    username_input: text_input::State,
    servername_input: text_input::State,
    port_input: text_input::State,
    password_input: text_input::State,
    connect_button: button::State,
    settings_button: button::State,
}

impl ConnectLayout {
    pub fn view(&mut self, current_style: &StyleTheme) -> Element<MainMessage> {
        Column::new()
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
            )
            .push(Column::new().height(Length::FillPortion(30)))
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
            .push(Column::new().height(Length::FillPortion(10)))
            .into()
    }
}
