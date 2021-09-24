// External.
use druid::widget::prelude::*;
use druid::widget::{
    Button, Container, CrossAxisAlignment, Flex, Label, Padding, SizedBox, TextBox,
};
use druid::{Data, Lens, LensExt, WidgetExt};
use sfml::audio::{Sound, SoundBuffer, SoundStatus};

// Std.
use std::thread;
use std::time::Duration;

use crate::misc::custom_data_button_controller::CustomButtonData;
// Custom.
use crate::misc::custom_text_box_controller::*;
use crate::misc::formatter_max_characters::*; // add formatter when #1975 is resolved
use crate::theme::BACKGROUND_SPECIAL_COLOR;
use crate::widgets::chat_list::*;
use crate::widgets::connected_list::*;
use crate::ApplicationState;
use crate::{global_params::*, Layout};

#[derive(Clone, Data, Lens)]
pub struct MainLayout {
    pub message: String,
    pub chat_list: ChatList,
    pub connected_list: ConnectedList,
    pub current_user_room: String,
    pub current_user_name: String,
    pub connected_count_text: usize,
}

impl MainLayout {
    pub fn new() -> Self {
        MainLayout {
            message: String::new(),
            connected_list: ConnectedList::new(),
            chat_list: ChatList::new(),
            current_user_room: String::new(),
            current_user_name: String::new(),
            connected_count_text: 0,
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        Padding::new(
            10.0,
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_flex_child(
                    Flex::column()
                        .must_fill_main_axis(true)
                        .with_flex_child(
                            Flex::row()
                                .with_child(
                                    Button::from_label(
                                        Label::new("settings").with_text_size(TEXT_SIZE),
                                    )
                                    .on_click(MainLayout::on_settings_clicked),
                                )
                                .expand(),
                            10.0,
                        )
                        .with_flex_child(Label::new("Text Chat").with_text_size(TEXT_SIZE), 10.0)
                        .with_default_spacer()
                        .with_flex_child(
                            Container::new(ChatList::build_ui())
                                .background(BACKGROUND_SPECIAL_COLOR)
                                .rounded(druid::theme::BUTTON_BORDER_RADIUS)
                                .expand(),
                            70.0,
                        )
                        .with_default_spacer()
                        .with_flex_child(
                            TextBox::multiline()
                                .with_placeholder("Type your message here...")
                                .with_text_size(TEXT_SIZE)
                                //.with_formatter(MaxCharactersFormatter::new(MAX_MESSAGE_SIZE))
                                .controller(CustomTextBoxController::new())
                                .lens(ApplicationState::main_layout.then(MainLayout::message))
                                .expand(),
                            10.0,
                        ),
                    60.0,
                )
                .with_default_spacer()
                .with_flex_child(
                    Flex::column()
                        .must_fill_main_axis(true)
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_flex_child(SizedBox::empty().expand(), 10.0)
                        .with_flex_child(
                            Label::new(|data: &ApplicationState, _env: &Env| {
                                format!("Connected: {}", data.main_layout.connected_count_text)
                            })
                            .with_text_size(TEXT_SIZE),
                            10.0,
                        )
                        .with_default_spacer()
                        .with_flex_child(
                            Container::new(ConnectedList::build_ui())
                                .background(BACKGROUND_SPECIAL_COLOR)
                                .rounded(druid::theme::BUTTON_BORDER_RADIUS)
                                .expand(),
                            80.0,
                        ),
                    40.0,
                ),
        )
    }
    pub fn set_user_talking(&mut self, username: &str, talk_start: bool) {
        let mut found = false;
        {
            let mut rooms_guard = self.connected_list.rooms.lock().unwrap();

            for room in rooms_guard.iter_mut() {
                let mut users_guard = room.users.lock().unwrap();

                for user in users_guard.iter_mut() {
                    if &user.user_data.username == username {
                        user.user_data.is_talking = talk_start;
                        found = true;
                        self.connected_list.refresh_ui = !self.connected_list.refresh_ui;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
        }

        if found == false {
            self.add_system_message(format!(
                "Warning: can't find user {}, at [{}:{}]",
                username,
                file!(),
                line!()
            ));
        }
    }
    pub fn play_connect_sound(&self) {
        thread::spawn(move || {
            let buffer = SoundBuffer::from_file(CONNECTED_SOUND_PATH).unwrap();
            let mut sound = Sound::with_buffer(&buffer);
            sound.play();
            while sound.status() == SoundStatus::PLAYING {
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }
    pub fn open_selected_user_info(&mut self, username: String) {
        self.connected_list.open_selected_user_info(username);
    }
    pub fn hide_user_info(&mut self) {
        self.connected_list.hide_user_info();
    }
    pub fn get_message_input(&self) -> String {
        self.message.clone()
    }
    pub fn set_user_ping(&mut self, username: &str, ping_ms: u16) -> Result<(), ()> {
        self.connected_list.set_user_ping(username, ping_ms)
    }
    pub fn clear_message_input(&mut self) {
        self.message.clear();
    }
    pub fn clear_text_chat(&mut self) {
        self.chat_list.clear_text_chat();
    }
    pub fn add_user(
        &mut self,
        username: String,
        room: String,
        ping_ms: u16,
        dont_show_notice: bool,
    ) -> Result<(), String> {
        if !dont_show_notice {
            self.chat_list
                .add_info_message(format!("{} just connected to the chat.", &username));

            if self.current_user_room == DEFAULT_ROOM_NAME {
                thread::spawn(move || {
                    let buffer = SoundBuffer::from_file(CONNECTED_SOUND_PATH).unwrap();
                    let mut sound = Sound::with_buffer(&buffer);
                    sound.play();
                    while sound.status() == SoundStatus::PLAYING {
                        std::thread::sleep(Duration::from_secs(1));
                    }
                });
            }
        }

        let res = self.connected_list.add_user(username, room, ping_ms);
        if let Err(msg) = res {
            return Err(format!("{} at [{}, {}]", msg, file!(), line!()));
        }

        self.connected_count_text += 1;

        Ok(())
    }
    pub fn get_room_count(&self) -> usize {
        self.connected_list.get_room_count()
    }
    pub fn add_room(&mut self, room_name: String) {
        self.connected_list.add_room(room_name);
    }
    pub fn move_user(&mut self, username: &str, room_to: &str) -> Result<(), String> {
        if self.current_user_name.is_empty() {
            panic!("self.current_user_name is empty");
        }

        if self.current_user_room.is_empty() {
            panic!("self.current_user_room is empty");
        }

        if let Err(msg) = self.connected_list.move_user(
            username,
            room_to,
            &self.current_user_name,
            &self.current_user_room,
        ) {
            Err(format!("{} at [{}, {}]", msg, file!(), line!()))
        } else {
            Ok(())
        }
    }
    pub fn remove_user(&mut self, username: &str) -> Result<(), String> {
        let mut removed_user_room = String::new();
        match self
            .connected_list
            .remove_user(username, &mut removed_user_room)
        {
            Err(msg) => return Err(format!("{} at [{}, {}]", msg, file!(), line!())),
            Ok(()) => {
                self.chat_list
                    .add_info_message(format!("{} disconnected from the chat.", username));

                if self.current_user_room == removed_user_room {
                    thread::spawn(move || {
                        let buffer = SoundBuffer::from_file(DISCONNECT_SOUND_PATH).unwrap();
                        let mut sound = Sound::with_buffer(&buffer);
                        sound.play();
                        while sound.status() == SoundStatus::PLAYING {
                            std::thread::sleep(Duration::from_secs(1));
                        }
                    });
                }

                self.connected_count_text -= 1;

                return Ok(());
            }
        }
    }
    pub fn add_message(&mut self, message: String, author: String, show_notification: bool) {
        self.chat_list.add_message(&message, &author);

        if (author != self.current_user_name) && show_notification {
            use notify_rust::Notification;
            #[cfg(target_os = "linux")]
            let icon_path = &format!(
                "{}/res/app_icon.png",
                std::env::current_dir().unwrap().to_str().unwrap()
            );
            #[cfg(target_os = "windows")]
            let icon_path = &format!(
                "{}\\res\\app_icon.png",
                std::env::current_dir().unwrap().to_str().unwrap()
            );
            Notification::new()
                .summary(&author)
                .body(&message)
                .icon(icon_path)
                .show()
                .unwrap();
        }
    }
    pub fn add_system_message(&mut self, message: String) {
        self.chat_list.add_system_message(message);
    }
    pub fn add_info_message(&mut self, message: String) {
        self.chat_list.add_info_message(message);
    }
    pub fn clear_all_users(&mut self) {
        self.connected_list.clear_all_users();
    }
    fn on_settings_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.current_layout = Layout::Settings;
    }
    pub fn send_message_event(data: &mut ApplicationState) {
        if !data.main_layout.message.is_empty() {
            // remove last '\n's
            while data.main_layout.message.chars().last().unwrap() == '\n' {
                data.main_layout.message.pop();
            }

            if data.main_layout.message.chars().count() > MAX_MESSAGE_SIZE {
                data.main_layout.add_system_message(format!(
                    "Your message is too long ({} characters when the limit is {})!",
                    data.main_layout.message.chars().count(),
                    MAX_MESSAGE_SIZE
                ));
                return;
            }

            if let Err(msg) = data
                .network_service
                .lock()
                .unwrap()
                .send_user_message(data.main_layout.get_message_input())
            {
                data.main_layout.add_system_message(msg.message);
            } else {
                data.main_layout.clear_message_input();
            }
        }
    }
    pub fn connect_list_item_pressed_event(
        data: &mut ApplicationState,
        button_info: &CustomButtonData,
    ) {
        if button_info.is_room {
            if data.main_layout.current_user_room != button_info.button_name {
                if let Err(msg) = data
                    .network_service
                    .lock()
                    .unwrap()
                    .enter_room(&button_info.button_name)
                {
                    data.main_layout.add_system_message(msg.message);
                }
            }
        } else {
            data.main_layout
                .open_selected_user_info(button_info.button_name.clone());
        }
    }
    pub fn user_volume_slider_moved_event(data: &mut ApplicationState) {
        // Apply to audio service.
        let audio_service_guard = data.audio_service.lock().unwrap();
        let users_audio = audio_service_guard.users_voice_data.lock().unwrap();
        for user in users_audio.iter() {
            let mut user_guard = user.lock().unwrap();
            if &user_guard.username
                == &data
                    .main_layout
                    .connected_list
                    .user_info_layout
                    .user_data
                    .username
            {
                user_guard.user_volume = data
                    .main_layout
                    .connected_list
                    .user_info_layout
                    .user_data
                    .volume as i32;
                break;
            }
        }

        // Apply to data.
        {
            let mut rooms_guard = data.main_layout.connected_list.rooms.lock().unwrap();

            let mut ok = false;

            for room in rooms_guard.iter_mut() {
                let mut users_guard = room.users.lock().unwrap();
                for user in users_guard.iter_mut() {
                    if user.user_data.username
                        == data
                            .main_layout
                            .connected_list
                            .user_info_layout
                            .user_data
                            .username
                    {
                        user.user_data.volume = data
                            .main_layout
                            .connected_list
                            .user_info_layout
                            .user_data
                            .volume;
                        ok = true;
                        break;
                    }
                }

                if ok {
                    break;
                }
            }
        }
    }
}
