// External.
use iced::{
    button, text_input, Align, Button, Color, Column, Element, HorizontalAlignment, Length, Row,
    Text, TextInput, VerticalAlignment,
};

// Custom.
use crate::themes::*;
use crate::widgets::chat_list::*;
use crate::widgets::users_list::*;
use crate::MainMessage;

#[derive(Debug, Clone)]
pub enum MainLayoutMessage {
    MessageInputEnterPressed,
    HideUserInfoPressed,
    UserItemPressed(usize),
}

#[derive(Debug, Default)]
pub struct MainLayout {
    pub chat_list: ChatList,
    users_list: UserList,

    pub connected_users: usize,

    pub message_string: String,

    message_input: text_input::State,
    settings_button: button::State,
}

impl MainLayout {
    pub fn open_selected_user_info(&mut self, id: usize) {
        self.users_list.open_selected_user_info(id);
    }
    pub fn hide_user_info(&mut self) {
        self.users_list.hide_user_info();
    }
    pub fn get_message_input(&self) -> String {
        self.message_string.clone()
    }
    pub fn clear_message_input(&mut self) {
        self.message_string.clear();
    }
    pub fn add_user(&mut self, username: String, dont_show_notice: bool) {
        self.users_list.add_user(username.clone());
        self.connected_users = self.users_list.get_user_count();
        if !dont_show_notice {
            self.add_info_message(format!("{} just connected to the chat.", username));
        }
    }
    pub fn remove_user(&mut self, username: String) -> Result<(), String> {
        match self.users_list.remove_user(username.clone()) {
            Err(msg) => return Err(format!("{} at [{}, {}]", msg, file!(), line!())),
            Ok(()) => {
                self.connected_users = self.users_list.get_user_count();
                self.add_info_message(format!("{} disconnected from the chat.", username));
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

        Row::new()
            .padding(10)
            .spacing(0)
            .align_items(Align::Center)
            .push(left.width(Length::FillPortion(65)))
            .push(right.width(Length::FillPortion(35)))
            .into()
    }
}
