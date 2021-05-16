// External.
use iced::{
    button, text_input, Align, Button, Color, Column, Element, HorizontalAlignment, Length, Row,
    Text, TextInput, VerticalAlignment,
};
use iced_aw::{modal, Card, Modal};
use rusty_audio::Audio;

// Std.
use std::thread;

// Custom.
use crate::global_params::*;
use crate::themes::*;
use crate::widgets::chat_list::*;
use crate::widgets::users_list::*;
use crate::MainMessage;

#[derive(Default, Debug)]
struct ModalState {
    message: String,
    cancel_state: button::State,
    ok_state: button::State,
}

#[derive(Clone, Debug)]
pub enum ModalMessage {
    CloseModal,
    OkButtonPressed,
}

#[derive(Debug, Clone)]
pub enum MainLayoutMessage {
    MessageInputEnterPressed,
    HideUserInfoPressed,
    UserItemPressed(String),
    RoomItemPressed(String),
}

#[derive(Debug)]
pub struct MainLayout {
    pub chat_list: ChatList,
    users_list: UserList,

    pub connected_users: usize,

    pub message_string: String,
    pub current_user_room: String,
    pub current_user_name: String,

    modal_state: modal::State<ModalState>,
    message_input: text_input::State,
    settings_button: button::State,
}

impl Default for MainLayout {
    fn default() -> Self {
        MainLayout {
            chat_list: ChatList::default(),
            users_list: UserList::default(),
            connected_users: 0,
            message_string: String::from(""),
            current_user_name: String::from(""),
            current_user_room: String::from(DEFAULT_ROOM_NAME),
            modal_state: modal::State::<ModalState>::default(),
            message_input: text_input::State::default(),
            settings_button: button::State::default(),
        }
    }
}

impl MainLayout {
    pub fn play_connect_sound(&self) {
        thread::spawn(move || {
            let mut audio = Audio::new();
            audio.add("sound", CONNECTED_SOUND_PATH);
            audio.play("sound"); // Execution continues while playback occurs in another thread.
            audio.wait(); // Block until sounds finish playing
        });
    }
    pub fn is_modal_window_showed(&self) -> bool {
        self.modal_state.is_shown()
    }
    pub fn show_modal_window(&mut self, message: String) {
        self.modal_state.inner_mut().message = message;
        self.modal_state.show(true);
    }
    pub fn hide_modal_window(&mut self) {
        self.modal_state.show(false);
    }
    pub fn open_selected_user_info(&mut self, username: String) {
        self.users_list.open_selected_user_info(username);
    }
    pub fn hide_user_info(&mut self) {
        self.users_list.hide_user_info();
    }
    pub fn get_message_input(&self) -> String {
        self.message_string.clone()
    }
    pub fn set_user_ping(&mut self, username: &str, ping_ms: u16) -> Result<(), ()> {
        self.users_list.set_user_ping(username, ping_ms)
    }
    pub fn clear_message_input(&mut self) {
        self.message_string.clear();
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
            self.add_info_message(format!("{} just connected to the chat.", &username));

            if self.current_user_room == DEFAULT_ROOM_NAME {
                thread::spawn(move || {
                    let mut audio = Audio::new();
                    audio.add("sound", CONNECTED_SOUND_PATH);
                    audio.play("sound"); // Execution continues while playback occurs in another thread.
                    audio.wait(); // Block until sounds finish playing
                });
            }
        }

        let res = self.users_list.add_user(username, room, ping_ms);
        if let Err(msg) = res {
            return Err(format!("{} at [{}, {}]", msg, file!(), line!()));
        }
        self.connected_users = self.users_list.get_user_count();

        Ok(())
    }
    pub fn add_room(&mut self, room_name: String) {
        self.users_list.add_room(room_name);
    }
    pub fn move_user(&mut self, username: &str, room_to: &str) -> Result<(), String> {
        if let Err(msg) = self.users_list.move_user(
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
            .users_list
            .remove_user(username, &mut removed_user_room)
        {
            Err(msg) => return Err(format!("{} at [{}, {}]", msg, file!(), line!())),
            Ok(()) => {
                self.connected_users = self.users_list.get_user_count();
                self.add_info_message(format!("{} disconnected from the chat.", username));

                if self.current_user_room == removed_user_room {
                    thread::spawn(move || {
                        let mut audio = Audio::new();
                        audio.add("sound", DISCONNECT_SOUND_PATH);
                        audio.play("sound"); // Execution continues while playback occurs in another thread.
                        audio.wait(); // Block until sounds finish playing
                    });
                }

                return Ok(());
            }
        }
    }
    pub fn add_message(&mut self, message: String, author: String) {
        self.chat_list.add_message(message, author);
    }
    pub fn add_system_message(&mut self, message: String) {
        self.chat_list.add_system_message(message);
    }
    pub fn add_info_message(&mut self, message: String) {
        self.chat_list.add_info_message(message);
    }
    pub fn clear_all_users(&mut self) {
        self.users_list.clear_all_users();
        self.connected_users = 0;
    }
    pub fn view(&mut self, current_style: &StyleTheme) -> Element<MainMessage> {
        let left: Column<MainMessage> = Column::new()
            .align_items(Align::Center)
            .padding(5)
            .spacing(5)
            .push(
                Row::new()
                    .push(
                        Button::new(
                            &mut self.settings_button,
                            Text::new("settings").color(Color::WHITE).size(18),
                        )
                        .on_press(MainMessage::ToSettingsButtonPressed)
                        .style(current_style.theme)
                        .width(Length::Shrink),
                    )
                    .height(Length::FillPortion(6)),
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
                        .on_submit(MainMessage::MessageFromMainLayout(
                            MainLayoutMessage::MessageInputEnterPressed,
                        ))
                        .size(22)
                        .style(current_style.theme),
                    )
                    .height(Length::FillPortion(7)),
            );

        let right: Column<MainMessage> = Column::new()
            .align_items(Align::Center)
            .padding(5)
            .spacing(5)
            .push(Row::new().height(Length::FillPortion(6)))
            .push(
                Text::new(format!("Connected: {}", self.connected_users))
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

        let content = Row::new()
            .padding(10)
            .spacing(0)
            .align_items(Align::Center)
            .push(left.width(Length::FillPortion(65)))
            .push(right.width(Length::FillPortion(35)));

        Modal::new(&mut self.modal_state, content, |state| {
            Card::new(
                Text::new("Information"),
                Text::new(&state.message), //Text::new("Zombie ipsum reversus ab viral inferno, nam rick grimes malum cerebro. De carne lumbering animata corpora quaeritis. Summus brains sit​​, morbo vel maleficia? De apocalypsi gorger omero undead survivor dictum mauris. Hi mindless mortuis soulless creaturas, imo evil stalking monstra adventus resi dentevil vultus comedat cerebella viventium. Qui animated corpse, cricket bat max brucks terribilem incessu zomby. The voodoo sacerdos flesh eater, suscitat mortuos comedere carnem virus. Zonbi tattered for solum oculi eorum defunctis go lum cerebro. Nescio brains an Undead zombies. Sicut malus putrid voodoo horror. Nigh tofth eliv ingdead.")
            )
            .foot(
                Row::new().spacing(10).padding(5).width(Length::Fill).push(
                    Button::new(
                        &mut state.ok_state,
                        Text::new("Ok").horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .width(Length::Fill)
                    .on_press(MainMessage::ModalWindowMessage(
                        ModalMessage::OkButtonPressed,
                    )),
                ),
            )
            .max_width(300)
            //.width(Length::Shrink)
            .on_close(MainMessage::ModalWindowMessage(ModalMessage::CloseModal))
            .into()
        })
        .backdrop(MainMessage::ModalWindowMessage(ModalMessage::CloseModal))
        .on_esc(MainMessage::ModalWindowMessage(ModalMessage::CloseModal))
        .into()
    }
}
