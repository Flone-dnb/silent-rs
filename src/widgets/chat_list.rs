// External.
use chrono::prelude::*;
use druid::widget::prelude::*;
use druid::widget::{CrossAxisAlignment, Flex, Label, LineBreaking, Padding, Scroll, ViewSwitcher};
use druid::{Color, Data, Lens};
use sfml::audio::{Sound, SoundBuffer, SoundStatus};

// Std.
use std::collections::LinkedList;
use std::rc::Rc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// Custom.
use crate::global_params::*;
use crate::ApplicationState;

#[derive(Clone, Data, Lens)]
pub struct ChatList {
    pub refresh_ui: bool, // because interior mutability (on messages) doesn't work in druid's data
    pub messages: Rc<Mutex<LinkedList<ChatMessage>>>,
    max_messages: usize,
}

impl ChatList {
    pub fn new() -> Self {
        ChatList {
            messages: Rc::new(Mutex::new(LinkedList::new())),
            max_messages: MAX_MESSAGES_ON_SCREEN,
            refresh_ui: false,
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        ViewSwitcher::new(
            |data: &ApplicationState, _env| data.main_layout.chat_list.refresh_ui,
            |selector, data, _env| match selector {
                _ => Box::new(ChatList::get_list_ui(data)),
            },
        )
    }
    fn get_list_ui(data: &ApplicationState) -> impl Widget<ApplicationState> {
        let mut content: Flex<ApplicationState> =
            Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

        let messages_guard = data.main_layout.chat_list.messages.lock().unwrap();
        for message in messages_guard.iter() {
            content.add_child(message.get_ui())
        }

        Scroll::new(content).vertical()
    }
    pub fn clear_text_chat(&mut self) {
        self.messages.lock().unwrap().clear();
        self.refresh_ui = !self.refresh_ui;
    }
    pub fn add_info_message(&mut self, message: String) {
        let mut messages_guard = self.messages.lock().unwrap();

        messages_guard.push_back(ChatMessage::new(
            message,
            String::from(""),
            MessageType::InfoMessage,
        ));

        if messages_guard.len() > self.max_messages {
            messages_guard.pop_front();
        }

        self.refresh_ui = !self.refresh_ui;
    }
    pub fn add_system_message(&mut self, message: String) {
        let mut messages_guard = self.messages.lock().unwrap();

        messages_guard.push_back(ChatMessage::new(
            message,
            String::from(""),
            MessageType::SystemMessage,
        ));

        if messages_guard.len() > self.max_messages {
            messages_guard.pop_front();
        }

        self.refresh_ui = !self.refresh_ui;
    }
    pub fn add_message(&mut self, message: &str, author: &str) {
        let mut messages_guard = self.messages.lock().unwrap();

        let mut add_message = true;

        if let Some(last_message) = messages_guard.back_mut() {
            if last_message.author == author && last_message.time == ChatMessage::current_time() {
                last_message.message.push('\n');
                last_message.message.push_str(&message);
                add_message = false;
            }
        }

        if add_message {
            messages_guard.push_back(ChatMessage::new(
                message.to_string(),
                author.to_string(),
                MessageType::UserMessage,
            ));

            if messages_guard.len() > self.max_messages {
                messages_guard.pop_front();
            }
        }

        self.refresh_ui = !self.refresh_ui;

        thread::spawn(move || {
            let buffer = SoundBuffer::from_file(NEW_MESSAGE_SOUND_PATH).unwrap();
            let mut sound = Sound::with_buffer(&buffer);
            sound.play();
            while sound.status() == SoundStatus::PLAYING {
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }
}

#[derive(Clone, Copy, Data, PartialEq)]
pub enum MessageType {
    UserMessage,
    SystemMessage,
    InfoMessage,
}

#[derive(Clone, Data)]
pub struct ChatMessage {
    message: String,
    author: String,
    pub time: String,
    message_type: MessageType,
}

impl ChatMessage {
    pub fn current_time() -> String {
        let now = Local::now();
        let mut hour: String = now.hour().to_string();
        let mut minute: String = now.minute().to_string();

        if hour.len() == 1 {
            hour = String::from("0") + &hour;
        }

        if minute.len() == 1 {
            minute = String::from("0") + &minute;
        }

        format!("{}:{}", hour, minute)
    }
    pub fn new(message: String, author: String, message_type: MessageType) -> Self {
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
            message_type,
        }
    }
    pub fn get_ui(&self) -> impl Widget<ApplicationState> {
        let mut _author: &str = &self.author;

        match self.message_type {
            MessageType::UserMessage => _author = &self.author,
            MessageType::SystemMessage => _author = "SYSTEM",
            MessageType::InfoMessage => _author = "INFO",
        }

        let mut message_column: Flex<ApplicationState> =
            Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

        message_column.add_child(
            Flex::row()
                .with_child(
                    Label::new(_author)
                        .with_text_size(MESSAGE_AUTHOR_TEXT_SIZE)
                        .with_text_color(druid::theme::BUTTON_DARK),
                )
                .with_child(
                    Label::new(format!("  {}", self.time.clone()))
                        .with_text_size(MESSAGE_TEXT_SIZE)
                        .with_text_color(Color::GRAY),
                ),
        );

        match self.message_type {
            MessageType::UserMessage => {
                message_column.add_child(
                    Label::new(self.message.clone())
                        .with_line_break_mode(LineBreaking::WordWrap)
                        .with_text_size(MESSAGE_TEXT_SIZE),
                );
            }
            MessageType::SystemMessage => {
                message_column.add_child(
                    Label::new(self.message.clone())
                        .with_text_size(MESSAGE_TEXT_SIZE)
                        .with_line_break_mode(LineBreaking::WordWrap)
                        .with_text_color(Color::RED),
                );
            }
            MessageType::InfoMessage => {
                message_column.add_child(
                    Label::new(self.message.clone())
                        .with_line_break_mode(LineBreaking::WordWrap)
                        .with_text_size(MESSAGE_TEXT_SIZE)
                        .with_text_color(Color::GRAY),
                );
            }
        }

        Padding::new(5.0, message_column)
    }
}
