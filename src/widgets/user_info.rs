// External.
use iced::{button, Button, Color, Column, Length, Text};

// Custom.
use super::users_list::UserItemData;
use crate::layouts::main_layout::MainLayoutMessage;
use crate::themes::StyleTheme;
use crate::MainMessage;

#[derive(Debug)]
pub struct UserInfo {
    user_data: UserItemData,

    return_button_state: button::State,
}

impl UserInfo {
    pub fn from(user_data: UserItemData) -> UserInfo {
        UserInfo {
            user_data,
            return_button_state: button::State::default(),
        }
    }
    pub fn update_data(&mut self, user_data: UserItemData) {
        self.user_data = user_data;
    }
    pub fn get_ui(&mut self, current_style: &StyleTheme) -> Column<MainMessage> {
        Column::new()
            .push(
                Text::new(&self.user_data.username)
                    .color(Color::WHITE)
                    .size(25),
            )
            .height(Length::Shrink)
            .push(
                Text::new(format!(
                    "Ping: {} ms",
                    self.user_data.ping_in_ms.to_string()
                ))
                .color(Color::WHITE)
                .height(Length::Shrink)
                .size(20),
            )
            .push(Column::new().height(Length::Fill))
            .push(
                Button::new(
                    &mut self.return_button_state,
                    Text::new("back").color(Color::WHITE).size(25),
                )
                .on_press(MainMessage::MessageFromMainLayout(
                    MainLayoutMessage::HideUserInfoPressed,
                ))
                .height(Length::Shrink)
                .style(current_style.theme),
            )
    }
}
