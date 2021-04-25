use chrono::prelude::*;
use iced::{scrollable, Color, Column, HorizontalAlignment, Length, Row, Scrollable, Text};
use std::collections::LinkedList;

use crate::MainMessage;

#[derive(Debug)]
pub struct ChatList {
    messages: LinkedList<ChatMessage>, // use list instead of vec because we will pop front to maintain 'max_messages' size
    max_messages: usize,

    scroll_state: scrollable::State,
}

impl ChatList {
    pub fn new() -> Self {
        ChatList {
            messages: LinkedList::default(),
            max_messages: 100,
            scroll_state: scrollable::State::default(),
        }
    }

    pub fn get_ui(&mut self) -> Scrollable<MainMessage> {
        let mut scroll_area = Scrollable::new(&mut self.scroll_state);
        for message in self.messages.iter() {
            scroll_area = scroll_area.push(message.get_ui());
        }

        scroll_area
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

    pub fn get_ui(&self) -> Column<MainMessage> {
        Column::new()
            .padding(10)
            .push(
                Row::new()
                    .push(
                        Text::new(&self.author)
                            .color(Color::from_rgb(
                                200 as f32 / 255.0,
                                40 as f32 / 255.0,
                                40 as f32 / 255.0,
                            ))
                            .size(23)
                            .horizontal_alignment(HorizontalAlignment::Left)
                            .width(Length::Shrink),
                    )
                    .push(
                        Text::new("  ")
                            .size(23)
                            .horizontal_alignment(HorizontalAlignment::Left)
                            .width(Length::Shrink),
                    )
                    .push(
                        Text::new(&self.time)
                            .color(Color::from_rgb(
                                128 as f32 / 255.0,
                                128 as f32 / 255.0,
                                128 as f32 / 255.0,
                            ))
                            .size(15)
                            .horizontal_alignment(HorizontalAlignment::Left)
                            .width(Length::Shrink),
                    ),
            )
            .push(Text::new(&self.message).color(Color::WHITE).size(22))
    }
}
