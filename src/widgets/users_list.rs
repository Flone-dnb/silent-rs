// External.
use chrono::prelude::*;
use iced::{
    button, scrollable, Button, Color, Container, HorizontalAlignment, Length, Row, Scrollable,
    Text,
};

// Std.
use std::sync::Mutex;

// Custom.
use crate::layouts::main_layout::MainLayoutMessage;
use crate::themes::*;
use crate::widgets::user_info::*;
use crate::MainMessage;

#[derive(Debug)]
pub struct UserList {
    users: Vec<UserItem>,
    user_info_layout: UserInfo,

    show_user_info: bool,

    process_lock: Mutex<()>,

    scroll_state: scrollable::State,
}

impl Default for UserList {
    fn default() -> Self {
        UserList {
            users: Vec::new(),
            process_lock: Mutex::new(()),
            scroll_state: scrollable::State::default(),
            user_info_layout: UserInfo::from(UserItemData::empty()),
            show_user_info: false,
        }
    }
}

impl UserList {
    pub fn open_selected_user_info(&mut self, id: usize) {
        let _process_guard = self.process_lock.lock().unwrap();

        if id < self.users.len() {
            self.user_info_layout
                .update_data(self.users[id].user_data.clone());
            self.show_user_info = true;
        }
    }
    pub fn hide_user_info(&mut self) {
        self.show_user_info = false;
    }
    pub fn clear_all_users(&mut self) {
        let _process_guard = self.process_lock.lock().unwrap();

        self.users.clear();
    }
    pub fn add_user(&mut self, username: String) {
        let _process_guard = self.process_lock.lock().unwrap();

        self.users.push(UserItem::new(username));
    }
    pub fn remove_user(&mut self, username: String) -> Result<(), String> {
        let _process_guard = self.process_lock.lock().unwrap();

        for (i, user) in self.users.iter().enumerate() {
            if user.user_data.username == username {
                self.users.remove(i);
                return Ok(());
            }
        }

        Err(format!(
            "An error occurred at UsersList::remove_user(), error: can't find user with name '{}' at [{}, {}]",
            username, file!(), line!()))
    }
    pub fn get_user_count(&self) -> usize {
        let _process_guard = self.process_lock.lock().unwrap();

        self.users.len()
    }
    pub fn get_ui(&mut self, current_style: &StyleTheme) -> Container<MainMessage> {
        let _process_guard = self.process_lock.lock().unwrap();

        if !self.show_user_info {
            let scroll_area = self.users.iter_mut().enumerate().fold(
                Scrollable::new(&mut self.scroll_state)
                    .width(Length::Fill)
                    .style(current_style.theme),
                |scroll_area, (i, user)| scroll_area.push(user.get_ui(i)),
            );

            Container::new(scroll_area)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
                .style(current_style.theme)
        } else {
            Container::new(self.user_info_layout.get_ui(current_style))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
                .style(current_style.theme)
        }
    }
}

#[derive(Debug)]
pub struct UserItemData {
    pub username: String,
    pub ping_in_ms: i32,
    pub connected_time_point: DateTime<Local>,
}

impl Clone for UserItemData {
    fn clone(&self) -> Self {
        UserItemData {
            username: self.username.clone(),
            ping_in_ms: self.ping_in_ms,
            connected_time_point: self.connected_time_point.clone(),
        }
    }
}

impl UserItemData {
    pub fn empty() -> Self {
        UserItemData {
            username: String::from(""),
            ping_in_ms: 0,
            connected_time_point: Local::now(),
        }
    }
}

#[derive(Debug)]
pub struct UserItem {
    user_data: UserItemData,

    pub button_state: button::State,
}

impl UserItem {
    pub fn new(username: String) -> Self {
        UserItem {
            user_data: UserItemData {
                username,
                ping_in_ms: 0,
                connected_time_point: Local::now(),
            },
            button_state: button::State::default(),
        }
    }

    pub fn get_ui(&mut self, id: usize) -> Button<MainMessage> {
        let content = Row::new()
            .push(
                Text::new(&self.user_data.username)
                    .color(Color::WHITE)
                    .size(23)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .width(Length::Shrink),
            )
            .push(
                Text::new(
                    String::from("  [") + &self.user_data.ping_in_ms.to_string()[..] + " ms]",
                )
                .color(Color::from_rgb(
                    128_f32 / 255.0,
                    128_f32 / 255.0,
                    128_f32 / 255.0,
                ))
                .size(17)
                .horizontal_alignment(HorizontalAlignment::Left)
                .width(Length::Shrink),
            );
        Button::new(&mut self.button_state, content)
            .width(Length::Fill)
            .style(default_theme::InteractiveTextButton)
            .on_press(MainMessage::MessageFromMainLayout(
                MainLayoutMessage::UserItemPressed(id),
            ))
    }
}
