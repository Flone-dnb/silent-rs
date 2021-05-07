use iced::{
    button, text_input, Align, Button, Color, Column, Element, HorizontalAlignment, Length, Row,
    Text, TextInput, VerticalAlignment,
};

use crate::themes::*;
use crate::MainMessage;

use crate::widgets::chat_list::*;
use crate::widgets::users_list::*;

#[derive(Debug, Default)]
pub struct MainLayout {
    pub chat_list: ChatList,
    users_list: UsersList,

    pub message_string: String,

    message_input: text_input::State,
    settings_button: button::State,
}

impl MainLayout {
    pub fn add_user(&mut self, username: String) {
        self.users_list.add_user(username);
    }
    pub fn add_message(&mut self, message: String, author: String) {
        self.chat_list.add_message(message, author);
    }
    pub fn add_system_message(&mut self, message: String) {
        self.chat_list.add_message(message, String::from(""));
    }
    pub fn view<'a>(&mut self, current_style: &StyleTheme) -> Element<MainMessage> {
        let left: Column<MainMessage> = Column::new()
            .align_items(Align::Center)
            .padding(5)
            .spacing(5)
            .push(
                Row::new()
                    .push(
                        Button::new(
                            &mut self.settings_button,
                            Text::new("settings").color(Color::WHITE).size(20),
                        )
                        .on_press(MainMessage::ToSettingsButtonPressed)
                        .style(current_style.theme)
                        .width(Length::Shrink),
                    )
                    .height(Length::FillPortion(5)),
            )
            .push(
                Text::new("Text Chat")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center)
                    .color(Color::WHITE)
                    .height(Length::FillPortion(5)),
            )
            .push(
                self.chat_list
                    .get_ui(current_style)
                    .height(Length::FillPortion(83)),
            )
            .push(
                Row::new()
                    .push(
                        TextInput::new(
                            &mut self.message_input,
                            "Type your message here...",
                            &self.message_string,
                            MainMessage::MessageInputChanged,
                        )
                        .size(22)
                        .style(current_style.theme),
                    )
                    .height(Length::FillPortion(7)),
            );

        let right: Column<MainMessage> = Column::new()
            .align_items(Align::Center)
            .padding(5)
            .spacing(5)
            .push(Row::new().height(Length::FillPortion(5)))
            .push(
                Text::new("Connected: 0")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center)
                    .color(Color::WHITE)
                    .height(Length::FillPortion(5)),
            )
            .push(
                self.users_list
                    .get_ui(current_style)
                    .width(Length::Fill)
                    .height(Length::FillPortion(90)),
            );

        Row::new()
            .padding(10)
            .spacing(0)
            .align_items(Align::Center)
            .push(left.width(Length::FillPortion(65)))
            .push(right.width(Length::FillPortion(35)))
            .into()
    }
}
