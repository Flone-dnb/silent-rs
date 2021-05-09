// External.
use iced::{button, Button, Color, Column, Container, Element, Length, Row, Text};

// Custom.
use crate::themes::*;
use crate::MainMessage;

#[derive(Debug, Default)]
pub struct SettingsLayout {
    active_option: CurrentActiveOption,

    about_details: AboutDetails,

    back_button: button::State,
    general_button: button::State,
    about_button: button::State,
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
        .on_press(MainMessage::GeneralSettingsButtonPressed)
        .width(Length::Fill)
        .height(Length::FillPortion(8));

        let mut about_button = Button::new(
            &mut self.about_button,
            Text::new("About").color(Color::WHITE),
        )
        .on_press(MainMessage::AboutSettingsButtonPressed)
        .width(Length::Fill)
        .height(Length::FillPortion(8));

        // Create right side content placeholder.
        let mut right_content_column: Column<MainMessage> = Column::new().padding(10).spacing(20);

        // Set styles for buttons.
        match self.active_option {
            CurrentActiveOption::General => {
                general_button = general_button.style(current_style.theme);
                about_button = about_button.style(default_theme::GrayButton);

                right_content_column =
                    right_content_column.push(Text::new("General settings...").color(Color::WHITE));
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
                            .on_press(MainMessage::GithubButtonPressed)
                            .style(current_style.theme)
                        )
                    );
            }
        }

        let content = Row::new()
            .padding(10)
            .spacing(20)
            .push(
                Container::new(
                    Column::new()
                        .padding(10)
                        .push(Column::new().height(Length::FillPortion(10)))
                        .push(general_button)
                        .push(Column::new().height(Length::FillPortion(5)))
                        .push(about_button)
                        .push(Column::new().height(Length::FillPortion(40)))
                        .push(
                            Button::new(
                                &mut self.back_button,
                                Text::new("Return").color(Color::WHITE),
                            )
                            .on_press(MainMessage::FromSettingsButtonPressed)
                            .width(Length::Fill)
                            .height(Length::FillPortion(8))
                            .style(current_style.theme),
                        )
                        .push(Column::new().height(Length::FillPortion(10))),
                )
                .width(Length::FillPortion(20))
                .height(Length::Fill)
                .style(current_style.theme),
            )
            .push(
                Container::new(right_content_column)
                    .width(Length::FillPortion(80))
                    .height(Length::Fill)
                    .style(current_style.theme),
            );

        Column::new().padding(10).push(content).into()
    }
}
