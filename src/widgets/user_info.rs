use druid::widget::prelude::*;
use druid::widget::{Button, CrossAxisAlignment, Flex, Label, LineBreaking, Slider};
use druid::{Color, Data, Lens, LensExt, WidgetExt};

use chrono::prelude::*;

use super::connected_list::UserItemData;
use crate::global_params::*;
use crate::layouts::main_layout::*;
use crate::misc::{custom_slider_controller::*, locale_keys::*};
use crate::widgets::connected_list::*;
use crate::ApplicationState;
use crate::CustomSliderID;

#[derive(Clone, Data, Lens)]
pub struct UserInfo {
    pub user_data: UserItemData,
}

impl UserInfo {
    pub fn from(user_data: UserItemData) -> UserInfo {
        UserInfo { user_data }
    }
    pub fn update_data(&mut self, user_data: UserItemData) {
        self.user_data = user_data;
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(
                Label::new(|data: &ApplicationState, _env: &Env| {
                    data.main_layout
                        .connected_list
                        .user_info_layout
                        .user_data
                        .username
                        .clone()
                })
                .with_text_size(TEXT_SIZE),
            )
            .with_default_spacer()
            .with_default_spacer()
            .with_child(
                Label::new(|data: &ApplicationState, _env: &Env| {
                    format!(
                        "{}: {} {}.",
                        data.localization
                            .get(LOCALE_MAIN_LAYOUT_USER_INFO_PING_TEXT)
                            .unwrap(),
                        data.main_layout
                            .connected_list
                            .user_info_layout
                            .user_data
                            .ping_ms,
                        data.localization
                            .get(LOCALE_MAIN_LAYOUT_USER_INFO_PING_TIME_TEXT)
                            .unwrap()
                    )
                })
                .with_text_size(TEXT_SIZE),
            )
            .with_default_spacer()
            .with_child(
                Label::new(|data: &ApplicationState, _env: &Env| {
                    let time_diff = Local::now()
                        - (*data
                            .main_layout
                            .connected_list
                            .user_info_layout
                            .user_data
                            .connected_time_point);
                    let mut _time_since_connected = String::new();
                    if time_diff.num_minutes() == 0 {
                        _time_since_connected = data
                            .localization
                            .get(LOCALE_MAIN_LAYOUT_USER_INFO_CONNECTED_JUST_NOW_TEXT)
                            .unwrap()
                            .clone();
                    } else if time_diff.num_hours() == 0 {
                        _time_since_connected = format!(
                            "{} {}",
                            time_diff.num_minutes(),
                            data.localization
                                .get(LOCALE_MAIN_LAYOUT_USER_INFO_CONNECTED_MIN_TEXT)
                                .unwrap()
                                .clone()
                        );
                    } else {
                        _time_since_connected = format!(
                            "{} {}",
                            time_diff.num_hours(),
                            data.localization
                                .get(LOCALE_MAIN_LAYOUT_USER_INFO_CONNECTED_HOUR_TEXT)
                                .unwrap()
                                .clone()
                        );
                    }

                    format!(
                        "{} {}.",
                        data.localization
                            .get(LOCALE_MAIN_LAYOUT_USER_INFO_CONNECTED_TEXT)
                            .unwrap(),
                        _time_since_connected
                    )
                })
                .with_text_size(TEXT_SIZE),
            )
            .with_default_spacer()
            .with_child(
                Label::new(|data: &ApplicationState, _env: &Env| {
                    format!(
                        "{}: {:.0} %",
                        data.localization
                            .get(LOCALE_MAIN_LAYOUT_USER_INFO_USER_VOLUME_TEXT)
                            .unwrap(),
                        data.main_layout
                            .connected_list
                            .user_info_layout
                            .user_data
                            .volume
                    )
                })
                .with_text_size(TEXT_SIZE),
            )
            .with_child(
                Label::new(|data: &ApplicationState, _env: &Env| {
                    format!(
                        "[{}]",
                        data.localization
                            .get(LOCALE_MAIN_LAYOUT_USER_INFO_USER_VOLUME_NOTE_TEXT)
                            .unwrap()
                    )
                })
                .with_line_break_mode(LineBreaking::WordWrap)
                .with_text_color(Color::GRAY)
                .with_text_size(TEXT_SIZE),
            )
            .with_child(
                Slider::new()
                    .with_step(1.0)
                    .with_range(0.0, 100.0)
                    .expand_width()
                    .controller(CustomSliderController::new(
                        CustomSliderID::UserVolumeSlider,
                    ))
                    .lens(
                        ApplicationState::main_layout.then(
                            MainLayout::connected_list.then(
                                ConnectedList::user_info_layout
                                    .then(UserInfo::user_data.then(UserItemData::volume)),
                            ),
                        ),
                    ),
            )
            .with_child(
                Button::from_label(
                    Label::new(|data: &ApplicationState, _env: &Env| {
                        data.localization
                            .get(LOCALE_MAIN_LAYOUT_USER_INFO_BACK_BUTTON_TEXT)
                            .unwrap()
                            .clone()
                    })
                    .with_text_size(TEXT_SIZE),
                )
                .on_click(UserInfo::on_back_clicked),
            )
    }
    fn on_back_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.main_layout.connected_list.hide_user_info();
    }
}
