// External.
use chrono::prelude::*;
use iced::{button, slider, Button, Color, Column, Length, Row, Slider, Text};

// Std.
use std::ops::RangeInclusive;

// Custom.
use super::users_list::UserItemData;
use crate::global_params::*;
use crate::layouts::main_layout::MainLayoutMessage;
use crate::themes::StyleTheme;
use crate::MainMessage;

#[derive(Debug)]
pub struct UserInfo {
    pub user_data: UserItemData,

    volume_slider_state: slider::State,
    return_button_state: button::State,
}

impl UserInfo {
    pub fn from(user_data: UserItemData) -> UserInfo {
        UserInfo {
            user_data,
            return_button_state: button::State::default(),
            volume_slider_state: slider::State::default(),
        }
    }
    pub fn update_data(&mut self, user_data: UserItemData) {
        self.user_data = user_data;
    }
    pub fn get_ui(&mut self, current_style: &StyleTheme) -> Column<MainMessage> {
        let time_diff = Local::now() - self.user_data.connected_time_point;
        let mut _time_since_connected = String::new();
        if time_diff.num_minutes() == 0 {
            _time_since_connected = String::from("just now.");
        } else if time_diff.num_hours() == 0 {
            _time_since_connected = format!("{} min. ago.", time_diff.num_minutes());
        } else {
            _time_since_connected = format!("{} h. ago.", time_diff.num_hours());
        }

        Column::new()
            .push(
                Text::new(&self.user_data.username)
                    .color(current_style.get_message_author_color())
                    .size(25),
            )
            .height(Length::Shrink)
            .push(
                Text::new(format!("ping: {} ms.", self.user_data.ping_ms.to_string()))
                    .color(Color::WHITE)
                    .height(Length::Shrink)
                    .size(22),
            )
            .push(
                Text::new(format!("connected: {}", _time_since_connected))
                    .color(Color::WHITE)
                    .height(Length::Shrink)
                    .size(22),
            )
            .push(
                Text::new("user volume:")
                    .size(22)
                    .height(Length::Shrink)
                    .color(Color::WHITE),
            )
            .push(
                Text::new("[this parameter will be reset after the restart]")
                    .size(18)
                    .height(Length::Shrink)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            )
            .push(
                Row::new()
                    .push(
                        Slider::new(
                            &mut self.volume_slider_state,
                            RangeInclusive::new(VOLUME_MIN, VOLUME_MAX),
                            self.user_data.volume as i32,
                            MainMessage::UserVolumeChanged,
                        )
                        .width(Length::FillPortion(50))
                        .style(current_style.theme),
                    )
                    .push(Column::new().width(Length::FillPortion(2)))
                    .push(
                        Text::new(format!("{}%", self.user_data.volume))
                            .size(22)
                            .height(Length::Shrink)
                            .width(Length::FillPortion(20)),
                    )
                    .push(Column::new().width(Length::FillPortion(28))),
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
