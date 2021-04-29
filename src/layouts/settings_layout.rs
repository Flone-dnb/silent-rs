use iced::{button, Button, Color, Column, Container, Element, Length, Row, Text};

use crate::themes::*;
use crate::MainMessage;

#[derive(Debug, Default)]
pub struct SettingsLayout {
    back_button: button::State,
}

impl SettingsLayout {
    pub fn view(&mut self, current_style: &StyleTheme) -> Element<MainMessage> {
        let content = Row::new()
            .padding(10)
            .spacing(20)
            .push(
                Container::new(
                    Column::new()
                        .padding(10)
                        .push(Column::new().height(Length::FillPortion(10)))
                        .push(Text::new("TODO").color(Color::WHITE))
                        .push(Column::new().height(Length::FillPortion(60)))
                        .push(
                            Button::new(
                                &mut self.back_button,
                                Text::new("Return").color(Color::WHITE),
                            )
                            .on_press(MainMessage::FromSettingsButtonPressed)
                            .style(current_style.theme),
                        )
                        .push(Column::new().height(Length::FillPortion(10))),
                )
                .width(Length::FillPortion(30))
                .height(Length::Fill)
                .style(current_style.theme),
            )
            .push(
                Container::new(
                    Column::new()
                        .padding(10)
                        .push(Column::new().height(Length::FillPortion(10)))
                        .push(Text::new("TODO").color(Color::WHITE))
                        .push(Column::new().height(Length::FillPortion(60)))
                        .push(Text::new("TODO").color(Color::WHITE))
                        .push(Column::new().height(Length::FillPortion(10))),
                )
                .width(Length::FillPortion(70))
                .height(Length::Fill)
                .style(current_style.theme),
            );

        Column::new().padding(10).push(content).into()
    }
}
