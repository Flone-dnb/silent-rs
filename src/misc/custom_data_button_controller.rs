use druid::widget::prelude::*;
use druid::widget::{Button, Controller};
use druid::{Selector, Target};

use crate::ApplicationState;

pub const CUSTOM_DATA_BUTTON_CLICKED: Selector<CustomButtonData> =
    Selector::new("custom_data_button_clicked");

#[derive(Clone)]
pub enum CustomButtonData {
    ConnectedListData { is_room: bool, button_name: String },
    MessageData { message: String },
}

pub struct CustomDataButtonController {
    data: CustomButtonData,
}

impl CustomDataButtonController {
    pub fn new(data: CustomButtonData) -> Self {
        CustomDataButtonController { data }
    }
}

impl Controller<ApplicationState, Button<ApplicationState>> for CustomDataButtonController {
    fn event(
        &mut self,
        child: &mut Button<ApplicationState>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut ApplicationState,
        env: &Env,
    ) {
        match event {
            Event::MouseUp(_) => {
                ctx.get_external_handle()
                    .submit_command(CUSTOM_DATA_BUTTON_CLICKED, self.data.clone(), Target::Auto)
                    .expect("failed to submit CUSTOM_SLIDER_ON_VALUE_CHANGED command");
                child.event(ctx, event, data, env)
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
