use iced::{
    text_input, Align, Color, Column, Element, HorizontalAlignment, Length, Row, Text, TextInput,
    VerticalAlignment,
};

use crate::themes::*;
use crate::MainMessage;

use crate::widgets::chat_list::*;
use crate::widgets::users_list::*;

#[derive(Debug, Default)]
pub struct MainLayout {
    chat_list: ChatList,
    users_list: UsersList,

    pub message_string: String,

    message_input: text_input::State,
}

impl MainLayout {
    pub fn view(&mut self, current_style: &StyleTheme) -> Element<MainMessage> {
        self.chat_list.add_message(
            String::from("Привет мир! Hello World!"),
            String::from("Bar"),
        );

        self.chat_list.add_message(
            String::from("Привет мир! Hello World!"),
            String::from("Foo"),
        );

        self.chat_list
            .add_message(String::from("Addition string!"), String::from("Foo"));

        self.users_list.add_user(String::from("Bar"));
        self.users_list.add_user(String::from("Foo"));

        let left: Column<MainMessage> = Column::new()
            .align_items(Align::Center)
            .padding(5)
            .spacing(10)
            .push(
                Text::new("Text Chat")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center)
                    .color(Color::WHITE)
                    .height(Length::FillPortion(8)),
            )
            .push(
                self.chat_list
                    .get_ui(current_style)
                    .height(Length::FillPortion(85)),
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
            .spacing(10)
            .push(
                Text::new("Connected: 0")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center)
                    .color(Color::WHITE)
                    .height(Length::FillPortion(8)),
            )
            .push(
                self.users_list
                    .get_ui(current_style)
                    .width(Length::Fill)
                    .height(Length::FillPortion(92)),
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
