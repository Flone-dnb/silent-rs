use iced::{scrollable, Color, HorizontalAlignment, Length, Row, Scrollable, Text};

use crate::MainMessage;

#[derive(Debug, Default)]
pub struct UsersList {
    users: Vec<UsersItem>,

    scroll_state: scrollable::State,
}

impl UsersList {
    pub fn get_ui(&mut self) -> Scrollable<MainMessage> {
        let mut scroll_area = Scrollable::new(&mut self.scroll_state);

        for entry in self.users.iter() {
            scroll_area = scroll_area.push(entry.get_ui());
        }

        scroll_area
    }

    pub fn add_user(&mut self, username: String) {
        self.users.push(UsersItem::new(username));
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
                Text::new("  ")
                    .size(23)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .width(Length::Shrink),
            )
            .push(
                Text::new(String::from("[") + &self.ping_in_ms.to_string()[..] + " ms]")
                    .color(Color::from_rgb(
                        128 as f32 / 255.0,
                        128 as f32 / 255.0,
                        128 as f32 / 255.0,
                    ))
                    .size(15)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .width(Length::Shrink),
            )
    }
}
