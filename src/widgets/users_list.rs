use iced::{scrollable, Color, Container, HorizontalAlignment, Length, Row, Scrollable, Text};

use crate::themes::*;
use crate::MainMessage;

#[derive(Debug, Default)]
pub struct UsersList {
    users: Vec<UsersItem>,

    scroll_state: scrollable::State,
}

impl UsersList {
    pub fn get_ui(&mut self, current_style: &StyleTheme) -> Container<MainMessage> {
        let scroll_area = self.users.iter().fold(
            Scrollable::new(&mut self.scroll_state)
                .width(Length::Fill)
                .style(current_style.theme),
            |scroll_area, user| scroll_area.push(user.get_ui()),
        );

        Container::new(scroll_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .style(current_style.theme)
    }
    pub fn clear_all_users(&mut self) {
        self.users.clear();
    }
    pub fn add_user(&mut self, username: String) {
        self.users.push(UsersItem::new(username));
    }
    pub fn get_user_count(&self) -> usize {
        self.users.len()
    }
}

#[derive(Debug, Default)]
pub struct UsersItem {
    username: String,
    ping_in_ms: i32,
}

impl UsersItem {
    pub fn new(username: String) -> Self {
        UsersItem {
            username,
            ping_in_ms: 0,
        }
    }

    pub fn get_ui(&self) -> Row<MainMessage> {
        Row::new()
            .push(
                Text::new(&self.username)
                    .color(Color::WHITE)
                    .size(23)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .width(Length::Shrink),
            )
            .push(
                Text::new(String::from("  [") + &self.ping_in_ms.to_string()[..] + " ms]")
                    .color(Color::from_rgb(
                        128 as f32 / 255.0,
                        128 as f32 / 255.0,
                        128 as f32 / 255.0,
                    ))
                    .size(17)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .width(Length::Shrink),
            )
    }
}
