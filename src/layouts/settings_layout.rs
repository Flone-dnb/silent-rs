use std::ops::RangeInclusive;

// External.
use iced::{button, slider, Button, Color, Column, Container, Element, Length, Row, Slider, Text};
use iced_native::Widget;

// Custom.
use crate::global_params::*;
use crate::themes::*;
use crate::MainMessage;

#[derive(Debug, Clone)]
pub enum SettingsLayoutMessage {
    GeneralSettingsButtonPressed,
    AboutSettingsButtonPressed,
    GithubButtonPressed,
    FromSettingsButtonPressed,
}

#[derive(Debug)]
pub struct SettingsLayout {
    active_option: CurrentActiveOption,

    about_details: AboutDetails,

    pub ui_scaling_slider_value: i32,

    back_button: button::State,
    general_button: button::State,
    about_button: button::State,
    ui_scaling_slider_state: slider::State,
}

impl Default for SettingsLayout {
    fn default() -> Self {
        SettingsLayout {
            ui_scaling_slider_value: 100,
            active_option: CurrentActiveOption::General,
            about_details: AboutDetails::default(),
            ui_scaling_slider_state: slider::State::default(),
            back_button: button::State::default(),
            general_button: button::State::default(),
            about_button: button::State::default(),
        }
    }
}

#[derive(Debug, Default)]
struct AboutDetails {
    github_button: button::State,
}

#[derive(Debug)]
pub enum CurrentActiveOption {
    General,
    About,
}

impl Default for CurrentActiveOption {
    fn default() -> Self {
        CurrentActiveOption::General
    }
}

impl SettingsLayout {
    pub fn set_active_option(&mut self, option: CurrentActiveOption) {
        self.active_option = option;
    }

    pub fn view(&mut self, current_style: &StyleTheme) -> Element<MainMessage> {
        // Create all buttons.
        let mut general_button = Button::new(
            &mut self.general_button,
            Text::new("General").color(Color::WHITE),
        )
        .on_press(MainMessage::MessageFromSettingsLayout(
            SettingsLayoutMessage::GeneralSettingsButtonPressed,
        ))
        .width(Length::Fill)
        .height(Length::FillPortion(8));

        let mut about_button = Button::new(
            &mut self.about_button,
            Text::new("About").color(Color::WHITE),
        )
        .on_press(MainMessage::MessageFromSettingsLayout(
            SettingsLayoutMessage::AboutSettingsButtonPressed,
        ))
        .width(Length::Fill)
        .height(Length::FillPortion(8));

        // Create right side content placeholder.
        let mut right_content_column: Column<MainMessage> = Column::new().padding(10).spacing(20);

        // Set styles for buttons.
        match self.active_option {
            CurrentActiveOption::General => {
                general_button = general_button.style(current_style.theme);
                about_button = about_button.style(default_theme::GrayButton);

                right_content_column = right_content_column
                    .push(Text::new("UI scaling:").color(Color::WHITE))
                    .push(
                        Row::new()
                            .push(
                                Slider::new(
                                    &mut self.ui_scaling_slider_state,
                                    RangeInclusive::new(UI_SCALING_MIN, UI_SCALING_MAX),
                                    self.ui_scaling_slider_value,
                                    MainMessage::UIScalingSliderMoved,
                                )
                                .width(Length::FillPortion(50)),
                            )
                            .push(Column::new().width(Length::FillPortion(2)))
                            .push(
                                Text::new(format!("{}%", self.ui_scaling_slider_value))
                                    .width(Length::FillPortion(20)),
                            )
                            .push(Column::new().width(Length::FillPortion(28))),
                    );
            }
            CurrentActiveOption::About => {
                general_button = general_button.style(default_theme::GrayButton);
                about_button = about_button.style(current_style.theme);

                right_content_column = right_content_column
                    .push(Text::new("Silent is a cross-platform, high-quality, low latency voice chat made for gaming.")
                            .color(Color::WHITE).size(25)
                    )
                    .push(Text::new(String::from("Version: ") + env!("CARGO_PKG_VERSION") + " (rs).")
                            .color(Color::WHITE).size(25)
                    )
                    .push(Row::new()
                        .push(Text::new("This is an open source project:  ").color(Color::WHITE).size(25))
                        .push(Button::new(
                                &mut self.about_details.github_button,
                                Text::new("source code").color(Color::WHITE).size(25),
                            )
                            .on_press(MainMessage::MessageFromSettingsLayout(SettingsLayoutMessage::GithubButtonPressed))
                            .style(current_style.theme)
                        )
                    );
            }
        }

        let content = Row::new()
            .padding(5)
            .spacing(15)
            .push(
                Container::new(
                    Column::new()
                        .padding(10)
                        .push(Column::new().height(Length::FillPortion(10)))
                        .push(general_button.height(Length::Shrink))
                        .push(Column::new().height(Length::FillPortion(5)))
                        .push(about_button.height(Length::Shrink))
                        .push(Column::new().height(Length::FillPortion(40)))
                        .push(
                            Button::new(
                                &mut self.back_button,
                                Text::new("back").color(Color::WHITE),
                            )
                            .on_press(MainMessage::MessageFromSettingsLayout(
                                SettingsLayoutMessage::FromSettingsButtonPressed,
                            ))
                            .width(Length::Fill)
                            .height(Length::Shrink)
                            .style(current_style.theme),
                        )
                        .push(Column::new().height(Length::FillPortion(10))),
                )
                .width(Length::FillPortion(13))
                .height(Length::Fill)
                .style(current_style.theme),
            )
            .push(
                Container::new(right_content_column)
                    .width(Length::FillPortion(87))
                    .height(Length::Fill)
                    .style(current_style.theme),
            );

        Column::new().padding(10).push(content).into()
    }
}
