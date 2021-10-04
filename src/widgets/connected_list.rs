// External.
use chrono::prelude::*;
use druid::widget::prelude::*;
use druid::widget::{
    Button, CrossAxisAlignment, Either, EnvScope, Flex, Label, Scroll, SizedBox, ViewSwitcher,
};
use druid::{Color, Data, Lens, TextAlignment, WidgetExt};
use sfml::audio::{Sound, SoundBuffer, SoundStatus};

// Std.
use std::collections::LinkedList;
use std::rc::Rc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// Custom.
use super::user_info::UserInfo;
use crate::global_params::*;
use crate::misc::{custom_data_button_controller::*, locale_keys::*};
use crate::ApplicationState;

#[derive(Clone, Data, Lens)]
pub struct ConnectedList {
    pub refresh_ui: bool, // because interior mutability (on rooms) doesn't work in druid's data
    pub rooms: Rc<Mutex<LinkedList<RoomItem>>>,
    pub is_showing_user_info: bool,
    pub user_info_layout: UserInfo,
}

impl ConnectedList {
    pub fn new() -> Self {
        ConnectedList {
            refresh_ui: false,
            rooms: Rc::new(Mutex::new(LinkedList::new())),
            is_showing_user_info: false,
            user_info_layout: UserInfo::from(UserItemData::empty()),
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        ViewSwitcher::new(
            |data: &ApplicationState, _env| data.main_layout.connected_list.refresh_ui,
            |selector, _data, _env| match selector {
                _ => Box::new(
                    Either::new(
                        |data: &ApplicationState, _env| {
                            data.main_layout.connected_list.is_showing_user_info
                        },
                        UserInfo::build_ui().expand(),
                        ConnectedList::build_list_ui(),
                    )
                    .expand(),
                ),
            },
        )
    }
    fn build_list_ui() -> impl Widget<ApplicationState> {
        Scroll::new(Flex::column().with_child(ViewSwitcher::new(
            // using ViewSwitcher as a trick to get to 'data', TODO: fix this
            |data: &ApplicationState, _env| data.current_layout,
            |selector, data, _env| match selector {
                _ => Box::new(ConnectedList::get_rooms_ui(data)),
            },
        )))
        .vertical()
    }
    fn get_rooms_ui(data: &ApplicationState) -> impl Widget<ApplicationState> {
        let mut column: Flex<ApplicationState> =
            Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

        let rooms_guard = data.main_layout.connected_list.rooms.lock().unwrap();
        for room in rooms_guard.iter() {
            column.add_child(room.get_ui(data));
        }

        column
    }
    pub fn open_selected_user_info(&mut self, username: String) {
        let rooms_guard = self.rooms.lock().unwrap();

        for room in rooms_guard.iter() {
            let users_guard = room.users.lock().unwrap();
            for user in users_guard.iter() {
                if user.user_data.username == username {
                    self.user_info_layout.update_data(user.user_data.clone());
                    self.is_showing_user_info = true;
                    self.refresh_ui = !self.refresh_ui;
                    return;
                }
            }
        }
    }
    pub fn hide_user_info(&mut self) {
        self.is_showing_user_info = false;

        self.refresh_ui = !self.refresh_ui;
    }
    pub fn clear_all_users(&mut self) {
        self.rooms.lock().unwrap().clear();

        self.refresh_ui = !self.refresh_ui;
    }
    pub fn get_room_count(&self) -> usize {
        self.rooms.lock().unwrap().len()
    }
    pub fn add_room(&mut self, room_name: String) {
        self.rooms
            .lock()
            .unwrap()
            .push_back(RoomItem::new(room_name));

        self.refresh_ui = !self.refresh_ui;
    }
    pub fn add_user(
        &mut self,
        username: String,
        room_name: String,
        ping_ms: u16,
    ) -> Result<(), String> {
        let mut rooms_guard = self.rooms.lock().unwrap();

        if room_name == "" {
            // Add to first room (lobby).
            let front = rooms_guard.front_mut();
            if front.is_some() {
                front.unwrap().add_user(username.clone(), ping_ms);
            } else {
                return Err(format!("An error occurred at UserList::add_user(), error: room with name '{}' not found at [{}, {}]", room_name, file!(), line!()));
            }
        } else {
            // Find room with this name
            let room_entry = rooms_guard
                .iter_mut()
                .find(|room_info| room_info.room_data.name == room_name);
            if room_entry.is_some() {
                room_entry.unwrap().add_user(username, ping_ms);
            } else {
                return Err(format!("An error occurred at UserList::add_user(), error: room with name '{}' not found at [{}, {}]", room_name, file!(), line!()));
            }
        }

        self.refresh_ui = !self.refresh_ui;

        Ok(())
    }
    pub fn set_user_ping(&mut self, username: &str, ping_ms: u16) -> Result<(), ()> {
        let mut rooms_guard = self.rooms.lock().unwrap();

        for room in rooms_guard.iter_mut() {
            let mut users_guard = room.users.lock().unwrap();
            for user in users_guard.iter_mut() {
                if user.user_data.username == username {
                    user.user_data.ping_ms = ping_ms;
                    self.refresh_ui = !self.refresh_ui;
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
        let mut rooms_guard = self.rooms.lock().unwrap();

        let mut removed = false;
        let mut removed_from_room = String::new();
        let mut user_data_clone = UserItemData::empty();

        for room in rooms_guard.iter_mut() {
            let mut users_guard = room.users.lock().unwrap();

            for (i, user) in users_guard.iter_mut().enumerate() {
                if user.user_data.username == username {
                    removed_from_room = room.room_data.name.clone();
                    user_data_clone = user.user_data.clone();
                    users_guard.remove(i);
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
        let room_entry = rooms_guard
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
                    let buffer = SoundBuffer::from_file(CONNECTED_SOUND_PATH).unwrap();
                    let mut sound = Sound::with_buffer(&buffer);
                    sound.play();
                    while sound.status() == SoundStatus::PLAYING {
                        std::thread::sleep(Duration::from_secs(1));
                    }
                });
            } else if removed_from_room == current_user_room {
                thread::spawn(move || {
                    let buffer = SoundBuffer::from_file(DISCONNECT_SOUND_PATH).unwrap();
                    let mut sound = Sound::with_buffer(&buffer);
                    sound.play();
                    while sound.status() == SoundStatus::PLAYING {
                        std::thread::sleep(Duration::from_secs(1));
                    }
                });
            }
        }

        self.refresh_ui = !self.refresh_ui;

        Ok(())
    }
    pub fn remove_user(
        &mut self,
        username: &str,
        removed_user_room: &mut String,
    ) -> Result<(), String> {
        let mut rooms_guard = self.rooms.lock().unwrap();

        for room in rooms_guard.iter_mut() {
            let mut users_guard = room.users.lock().unwrap();

            for (i, user) in users_guard.iter_mut().enumerate() {
                if user.user_data.username == username {
                    *removed_user_room = room.room_data.name.clone();
                    users_guard.remove(i);
                    self.refresh_ui = !self.refresh_ui;
                    return Ok(());
                }
            }
        }

        Err(format!(
            "An error occurred at UserList::remove_user(), error: can't find user with name '{}' at [{}, {}]",
            username, file!(), line!()))
    }
    pub fn get_user_count(&self) -> usize {
        let rooms_guard = self.rooms.lock().unwrap();

        let mut user_count = 0;
        for room in rooms_guard.iter() {
            let users_guard = room.users.lock().unwrap();
            user_count += users_guard.len();
        }

        user_count
    }
}

#[derive(Clone, Data)]
pub struct RoomItem {
    pub room_data: RoomItemData,
    pub users: Rc<Mutex<LinkedList<UserItem>>>,
}

impl RoomItem {
    pub fn new(room_name: String) -> Self {
        RoomItem {
            room_data: RoomItemData { name: room_name },
            users: Rc::new(Mutex::new(LinkedList::new())),
        }
    }
    pub fn add_user(&mut self, username: String, ping_ms: u16) {
        self.users
            .lock()
            .unwrap()
            .push_back(UserItem::new(username, ping_ms));
    }
    pub fn add_user_from_user_data(&mut self, user_data: UserItemData) {
        self.users
            .lock()
            .unwrap()
            .push_back(UserItem::new_from_data(user_data))
    }
    pub fn get_ui(&self, data: &ApplicationState) -> impl Widget<ApplicationState> {
        let mut column: Flex<ApplicationState> =
            Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

        // add room name first
        column.add_child(
            Button::from_label(Label::new(self.room_data.name.clone()).with_text_size(TEXT_SIZE))
                .controller(CustomDataButtonController::new(
                    CustomButtonData::ConnectedListData {
                        is_room: true,
                        button_name: self.room_data.name.clone(),
                    },
                )),
        );

        // then add users
        let users_guard = self.users.lock().unwrap();
        for user in users_guard.iter() {
            column.add_child(user.get_ui(data));
        }

        column
    }
}

#[derive(Clone, Data)]
pub struct RoomItemData {
    pub name: String,
}

#[derive(Clone, Data)]
pub struct UserItem {
    pub user_data: UserItemData,
}

impl UserItem {
    pub fn new(username: String, ping_ms: u16) -> Self {
        UserItem {
            user_data: UserItemData {
                username,
                ping_ms,
                volume: 100.0,
                is_talking: false,
                connected_time_point: Rc::new(Local::now()),
            },
        }
    }
    pub fn new_from_data(user_data: UserItemData) -> Self {
        UserItem { user_data }
    }
    pub fn get_ui(&self, data: &ApplicationState) -> impl Widget<ApplicationState> {
        let mut row: Flex<ApplicationState> = Flex::row()
            .must_fill_main_axis(true)
            .with_child(SizedBox::new(Label::new("  ").with_text_size(TEXT_SIZE)))
            .with_child(SizedBox::new(Label::new("  ").with_text_size(TEXT_SIZE)));

        // add user name
        let mut user_label: Label<ApplicationState> =
            Label::new(self.user_data.username.clone()).with_text_size(TEXT_SIZE);

        if self.user_data.is_talking {
            user_label.set_text_color(data.theme.button_dark_color.clone());
        }

        row.add_child(EnvScope::new(
            |env, _data| {
                env.set(druid::theme::BUTTON_DARK, Color::rgba8(0, 0, 0, 0));
                env.set(druid::theme::BUTTON_LIGHT, Color::rgba8(0, 0, 0, 0));
            },
            Button::from_label(user_label).controller(CustomDataButtonController::new(
                CustomButtonData::ConnectedListData {
                    is_room: false,
                    button_name: self.user_data.username.clone(),
                },
            )),
        ));

        // add user ping
        let user_ping = self.user_data.ping_ms;
        row.add_child(
            Label::new(move |data: &ApplicationState, _env: &Env| {
                format!(
                    "{} {}",
                    user_ping,
                    data.localization
                        .get(LOCALE_MAIN_LAYOUT_USER_INFO_PING_TIME_TEXT)
                        .unwrap()
                )
            })
            .with_text_size(MESSAGE_AUTHOR_TEXT_SIZE)
            .with_text_color(Color::GRAY)
            .with_text_alignment(TextAlignment::End),
        );

        row
    }
}

#[derive(Data, Lens)]
pub struct UserItemData {
    pub username: String,
    pub ping_ms: u16,
    pub volume: f64,
    pub is_talking: bool,
    pub connected_time_point: Rc<DateTime<Local>>, // using Rc because DateTime does not implement Clone
}

impl Clone for UserItemData {
    fn clone(&self) -> Self {
        UserItemData {
            username: self.username.clone(),
            ping_ms: self.ping_ms,
            volume: self.volume,
            is_talking: self.is_talking,
            connected_time_point: Rc::new((*self.connected_time_point).clone()),
        }
    }
}

impl UserItemData {
    pub fn empty() -> Self {
        UserItemData {
            username: String::from(""),
            ping_ms: 0,
            is_talking: false,
            volume: 100.0,
            connected_time_point: Rc::new(Local::now()),
        }
    }
}
