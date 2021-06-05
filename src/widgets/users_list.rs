// External.
use chrono::prelude::*;
use iced::{
    button, scrollable, Button, Color, Column, Container, HorizontalAlignment, Length, Row,
    Scrollable, Text,
};
use rusty_audio::Audio;

// Std.
use std::collections::LinkedList;
use std::sync::Mutex;
use std::thread;

// Custom.
use crate::global_params::*;
use crate::layouts::main_layout::MainLayoutMessage;
use crate::themes::*;
use crate::widgets::user_info::*;
use crate::MainMessage;

#[derive(Debug)]
pub struct UserList {
    pub rooms: LinkedList<RoomItem>,
    pub user_info_layout: UserInfo,

    show_user_info: bool,

    pub process_lock: Mutex<()>,

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
    pub fn add_user(
        &mut self,
        username: String,
        room_name: String,
        ping_ms: u16,
    ) -> Result<(), String> {
        let _process_guard = self.process_lock.lock().unwrap();

        if room_name == "" {
            // Add to first room (lobby).
            let front = self.rooms.front_mut();
            if front.is_some() {
                front.unwrap().add_user(username.clone(), ping_ms);
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
                room_entry.unwrap().add_user(username, ping_ms);
            } else {
                return Err(format!("An error occurred at UserList::add_user(), error: room with name '{}' not found at [{}, {}]", room_name, file!(), line!()));
            }
        }

        Ok(())
    }
    pub fn set_user_ping(&mut self, username: &str, ping_ms: u16) -> Result<(), ()> {
        let _process_guard = self.process_lock.lock().unwrap();

        for room in self.rooms.iter_mut() {
            for user in room.users.iter_mut() {
                if user.user_data.username == username {
                    user.user_data.ping_ms = ping_ms;
                    return Ok(());
                }
            }
        }

        Err(()) // not found
    }
    pub fn move_user(
        &mut self,
        username: &str,
        room_to: &str,
        current_user_name: &str,
        current_user_room: &str,
    ) -> Result<(), String> {
        let _process_guard = self.process_lock.lock().unwrap();

        let mut removed = false;
        let mut removed_from_room = String::new();
        let mut user_data_clone = UserItemData::empty();

        for room in self.rooms.iter_mut() {
            for (i, user) in room.users.iter_mut().enumerate() {
                if user.user_data.username == username {
                    removed_from_room = room.room_data.name.clone();
                    user_data_clone = user.user_data.clone();
                    room.users.remove(i);
                    removed = true;
                    break;
                }
            }
            if removed {
                break;
            }
        }

        if !removed {
            return Err(format!(
                "An error occurred at UserList::move_user(), error: can't find user with name '{}' at [{}, {}]",
                username, file!(), line!()));
        }

        // Find room with this name
        let room_entry = self
            .rooms
            .iter_mut()
            .find(|room_info| room_info.room_data.name == room_to);
        if room_entry.is_some() {
            room_entry.unwrap().add_user_from_user_data(user_data_clone);
        } else {
            return Err(format!("An error occurred at UserList::move_user(), error: room with name '{}' not found at [{}, {}]", room_to, file!(), line!()));
        }

        if username != current_user_name {
            if room_to == current_user_room {
                thread::spawn(move || {
                    let mut audio = Audio::new();
                    audio.add("sound", CONNECTED_SOUND_PATH);
                    audio.play("sound"); // Execution continues while playback occurs in another thread.
                    audio.wait(); // Block until sounds finish playing
                });
            } else if removed_from_room == current_user_room {
                thread::spawn(move || {
                    let mut audio = Audio::new();
                    audio.add("sound", DISCONNECT_SOUND_PATH);
                    audio.play("sound"); // Execution continues while playback occurs in another thread.
                    audio.wait(); // Block until sounds finish playing
                });
            }
        }

        Ok(())
    }
    pub fn remove_user(
        &mut self,
        username: &str,
        removed_user_room: &mut String,
    ) -> Result<(), String> {
        let _process_guard = self.process_lock.lock().unwrap();

        for room in self.rooms.iter_mut() {
            for (i, user) in room.users.iter_mut().enumerate() {
                if user.user_data.username == username {
                    *removed_user_room = room.room_data.name.clone();
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
                |scroll_area, room| scroll_area.push(room.get_ui(&current_style)),
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
    pub users: LinkedList<UserItem>,

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
    pub fn add_user(&mut self, username: String, ping_ms: u16) {
        self.users.push_back(UserItem::new(username, ping_ms));
    }
    pub fn add_user_from_user_data(&mut self, user_data: UserItemData) {
        self.users.push_back(UserItem::new_from_data(user_data))
    }
    pub fn get_ui(&mut self, current_style: &StyleTheme) -> Column<MainMessage> {
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

        self.users.iter_mut().fold(column, |column, user| {
            column.push(user.get_ui(&current_style))
        })
    }
}

#[derive(Debug)]
pub struct RoomItemData {
    pub name: String,
}

#[derive(Debug)]
pub struct UserItem {
    pub user_data: UserItemData,

    pub button_state: button::State,
}

impl UserItem {
    pub fn new(username: String, ping_ms: u16) -> Self {
        UserItem {
            user_data: UserItemData {
                username,
                ping_ms,
                volume: 100,
                is_talking: false,
                connected_time_point: Local::now(),
            },
            button_state: button::State::default(),
        }
    }
    pub fn new_from_data(user_data: UserItemData) -> Self {
        UserItem {
            user_data,
            button_state: button::State::default(),
        }
    }
    pub fn get_ui(&mut self, current_style: &StyleTheme) -> Button<MainMessage> {
        let mut content = Row::new().push(Text::new("    "));

        if self.user_data.is_talking {
            content = content.push(
                Text::new(&self.user_data.username)
                    .color(current_style.get_message_author_color())
                    .size(23)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .width(Length::Shrink),
            )
        } else {
            content = content.push(
                Text::new(&self.user_data.username)
                    .color(Color::WHITE)
                    .size(23)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .width(Length::Shrink),
            );
        }

        content = content.push(
            Text::new(String::from("  [") + &self.user_data.ping_ms.to_string()[..] + " ms]")
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
    pub ping_ms: u16,
    pub volume: u16,
    pub is_talking: bool,
    pub connected_time_point: DateTime<Local>,
}

impl Clone for UserItemData {
    fn clone(&self) -> Self {
        UserItemData {
            username: self.username.clone(),
            ping_ms: self.ping_ms,
            volume: self.volume,
            is_talking: self.is_talking,
            connected_time_point: self.connected_time_point,
        }
    }
}

impl UserItemData {
    pub fn empty() -> Self {
        UserItemData {
            username: String::from(""),
            ping_ms: 0,
            is_talking: false,
            volume: 100,
            connected_time_point: Local::now(),
        }
    }
}
