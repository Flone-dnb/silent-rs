use druid::widget::prelude::*;
use druid::widget::{Button, CrossAxisAlignment, Flex, Label, LineBreaking, Slider};
use druid::{Data, Lens, LensExt, Color, WidgetExt};

use chrono::prelude::*;

use super::connected_list::UserItemData;
use crate::global_params::*;
use crate::layouts::main_layout::*;
use crate::misc::custom_slider_controller::*;
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
                        "ping: {} ms",
                        data.main_layout
                            .connected_list
                            .user_info_layout
                            .user_data
                            .ping_ms
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
                        _time_since_connected = String::from("just now.");
                    } else if time_diff.num_hours() == 0 {
                        _time_since_connected = format!("{} min. ago.", time_diff.num_minutes());
                    } else {
                        _time_since_connected = format!("{} h. ago.", time_diff.num_hours());
                    }

                    format!("connected: {}", _time_since_connected)
                })
                .with_text_size(TEXT_SIZE),
            )
            .with_default_spacer()
            .with_child(
                Label::new(|data: &ApplicationState, _env: &Env| {
                    format!(
                        "user volume: {:.0} %",
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
                Label::new("[this parameter will be reset after the restart]")
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
                Button::from_label(Label::new("Back").with_text_size(TEXT_SIZE))
                    .on_click(UserInfo::on_back_clicked),
            )
    }
    fn on_back_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.main_layout.connected_list.hide_user_info();
    }
}
