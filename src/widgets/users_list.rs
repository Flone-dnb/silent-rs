// External.
use chrono::prelude::*;
use iced::{
    button, scrollable, Button, Color, Column, Container, HorizontalAlignment, Length, Row,
    Scrollable, Text,
};

// Std.
use std::collections::LinkedList;
use std::sync::Mutex;

// Custom.
use crate::layouts::main_layout::MainLayoutMessage;
use crate::themes::*;
use crate::widgets::user_info::*;
use crate::MainMessage;

#[derive(Debug)]
pub struct UserList {
    rooms: LinkedList<RoomItem>,
    user_info_layout: UserInfo,

    show_user_info: bool,

    process_lock: Mutex<()>,

    scroll_state: scrollable::State,
}

impl Default for UserList {
    fn default() -> Self {
        UserList {
            rooms: LinkedList::new(),
            process_lock: Mutex::new(()),
            scroll_state: scrollable::State::default(),
            user_info_layout: UserInfo::from(UserItemData::empty()),
            show_user_info: false,
        }
    }
}

impl UserList {
    pub fn open_selected_user_info(&mut self, username: String) {
        let _process_guard = self.process_lock.lock().unwrap();

        for room in self.rooms.iter() {
            for user in room.users.iter() {
                if user.user_data.username == username {
                    self.user_info_layout.update_data(user.user_data.clone());
                    self.show_user_info = true;
                    return;
                }
            }
        }
    }
    pub fn hide_user_info(&mut self) {
        self.show_user_info = false;
    }
    pub fn clear_all_users(&mut self) {
        let _process_guard = self.process_lock.lock().unwrap();

        self.rooms.clear();
    }
    pub fn add_room(&mut self, room_name: String) {
        let _process_guard = self.process_lock.lock().unwrap();

        self.rooms.push_back(RoomItem::new(room_name));
    }
    pub fn add_user(&mut self, username: String, room_name: String) -> Result<(), String> {
        let _process_guard = self.process_lock.lock().unwrap();

        if room_name == "" {
            // Add to first room (lobby).
            let front = self.rooms.front_mut();
            if front.is_some() {
                front.unwrap().add_user(username.clone());
            } else {
                return Err(format!("An error occurred at UserList::add_user(), error: room with name '{}' not found at [{}, {}]", room_name, file!(), line!()));
            }
        } else {
            // Find room with this name
            let room_entry = self
                .rooms
                .iter_mut()
                .find(|room_info| room_info.room_data.name == room_name);
            if room_entry.is_some() {
                room_entry.unwrap().add_user(username);
            } else {
                return Err(format!("An error occurred at UserList::add_user(), error: room with name '{}' not found at [{}, {}]", room_name, file!(), line!()));
            }
        }

        Ok(())
    }
    pub fn remove_user(&mut self, username: String) -> Result<(), String> {
        let _process_guard = self.process_lock.lock().unwrap();

        for room in self.rooms.iter_mut() {
            for (i, user) in room.users.iter_mut().enumerate() {
                if user.user_data.username == username {
                    room.users.remove(i);
                    return Ok(());
                }
            }
        }

        Err(format!(
            "An error occurred at UserList::remove_user(), error: can't find user with name '{}' at [{}, {}]",
            username, file!(), line!()))
    }
    pub fn get_user_count(&self) -> usize {
        let _process_guard = self.process_lock.lock().unwrap();

        let mut user_count = 0;
        for room in self.rooms.iter() {
            user_count += room.users.len();
        }

        user_count
    }
    pub fn get_ui(&mut self, current_style: &StyleTheme) -> Container<MainMessage> {
        let _process_guard = self.process_lock.lock().unwrap();

        if !self.show_user_info {
            let scroll_area = self.rooms.iter_mut().fold(
                Scrollable::new(&mut self.scroll_state)
                    .width(Length::Fill)
                    .style(current_style.theme),
                |scroll_area, room| scroll_area.push(room.get_ui()),
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
pub struct RoomItem {
    room_data: RoomItemData,
    users: LinkedList<UserItem>,

    pub button_state: button::State,
}

impl RoomItem {
    pub fn new(room_name: String) -> Self {
        RoomItem {
            room_data: RoomItemData { name: room_name },
            users: LinkedList::new(),
            button_state: button::State::default(),
        }
    }
    pub fn add_user(&mut self, username: String) {
        self.users.push_back(UserItem::new(username));
    }
    pub fn get_ui(&mut self) -> Column<MainMessage> {
        let room_row = Row::new().push(
            Text::new(&self.room_data.name)
                .color(Color::WHITE)
                .size(23)
                .horizontal_alignment(HorizontalAlignment::Left)
                .width(Length::Shrink),
        );

        let room_button = Button::new(&mut self.button_state, room_row)
            .width(Length::Fill)
            .style(default_theme::InteractiveTextButton)
            .on_press(MainMessage::MessageFromMainLayout(
                MainLayoutMessage::RoomItemPressed(self.room_data.name.clone()),
            ));

        let column = Column::new().width(Length::Fill).push(room_button);

        self.users
            .iter_mut()
            .fold(column, |column, user| column.push(user.get_ui()))
    }
}

#[derive(Debug)]
pub struct RoomItemData {
    pub name: String,
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
    pub fn get_ui(&mut self) -> Button<MainMessage> {
        let content = Row::new()
            .push(Text::new("    "))
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
                MainLayoutMessage::UserItemPressed(self.user_data.username.clone()),
            ))
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
            connected_time_point: self.connected_time_point,
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
