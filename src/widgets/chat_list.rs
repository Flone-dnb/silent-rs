use chrono::prelude::*;
use iced::{
    scrollable, Color, Column, Container, HorizontalAlignment, Length, Row, Scrollable, Text,
    VerticalAlignment,
};
use std::collections::LinkedList;

use crate::themes::*;
use crate::MainMessage;

#[derive(Debug)]
pub struct ChatList {
    messages: LinkedList<ChatMessage>, // use list instead of vec because we will pop front to maintain 'max_messages' size
    max_messages: usize,

    scroll_state: scrollable::State,
}

impl Default for ChatList {
    fn default() -> Self {
        ChatList::new()
    }
}

impl ChatList {
    pub fn new() -> Self {
        ChatList {
            messages: LinkedList::default(),
            max_messages: 100,
            scroll_state: scrollable::State::default(),
        }
    }

    pub fn get_ui(&mut self, current_style: &StyleTheme) -> Container<MainMessage> {
        let scroll_area = self.messages.iter().fold(
            Scrollable::new(&mut self.scroll_state)
                .width(Length::Fill)
                .style(current_style.theme),
            |scroll_area, message| scroll_area.push(message.get_ui(&current_style)),
        );

        Container::new(scroll_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .style(current_style.theme)
    }

    pub fn add_message(&mut self, message: String, author: String) {
        let mut same_author = false;

        if let Some(last_message) = self.messages.back_mut() {
            if last_message.author == author {
                last_message.message.push_str("\n");
                last_message.message.push_str(&message);

                same_author = true;
            }
        }

        if same_author == false {
            self.messages.push_back(ChatMessage::new(message, author));
        }
    }
}

#[derive(Debug, Default)]
pub struct ChatMessage {
    message: String,
    author: String,
    time: String,
}

impl ChatMessage {
    pub fn new(message: String, author: String) -> Self {
        let now = Local::now();
        let mut hour: String = now.hour().to_string();
        let mut minute: String = now.minute().to_string();

        if hour.len() == 1 {
            hour = String::from("0") + &hour;
        }

        if minute.len() == 1 {
            minute = String::from("0") + &minute;
        }

        ChatMessage {
            message,
            author,
            time: format!("{}:{}", hour, minute),
        }
    }

    pub fn get_ui(&self, current_style: &StyleTheme) -> Column<MainMessage> {
        Column::new()
            .padding(10)
            .push(
                Row::new()
                    .push(
                        Text::new(&self.author)
                            .color(current_style.get_message_author_color())
                            .size(23)
                            .horizontal_alignment(HorizontalAlignment::Left)
                            .vertical_alignment(VerticalAlignment::Top)
                            .width(Length::Shrink),
                    )
                    .push(
                        Text::new(String::from("  ") + &self.time)
                            .color(Color::from_rgb(
                                128 as f32 / 255.0,
                                128 as f32 / 255.0,
                                128 as f32 / 255.0,
                            ))
                            .size(17)
                            .horizontal_alignment(HorizontalAlignment::Left)
                            .vertical_alignment(VerticalAlignment::Bottom)
                            .width(Length::Shrink),
                    ),
            )
            .push(Text::new(&self.message).color(Color::WHITE).size(22))
    }
}
