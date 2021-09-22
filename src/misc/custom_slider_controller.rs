use druid::widget::prelude::*;
use druid::widget::{Controller, SizedBox};
use druid::{Selector, Target};

use crate::CustomSliderID;

pub const CUSTOM_SLIDER_ON_VALUE_CHANGED: Selector<OnCustomSliderMovedInfo> =
    Selector::new("custom_slider_on_value_changed");

pub struct OnCustomSliderMovedInfo {
    pub custom_slider_id: CustomSliderID,
    pub value: u16,
}

pub struct CustomSliderController {
    is_lmb_pressed: bool,
    pub custom_slider_id: CustomSliderID,
}

impl CustomSliderController {
    pub fn new(custom_slider_id: CustomSliderID) -> Self {
        CustomSliderController {
            is_lmb_pressed: false,
            custom_slider_id,
        }
    }
}

impl Controller<f64, SizedBox<f64>> for CustomSliderController {
    fn event(
        &mut self,
        child: &mut SizedBox<f64>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut f64,
        env: &Env,
    ) {
        match event {
            Event::MouseDown(ev) if ev.buttons.has_left() => {
                self.is_lmb_pressed = true;

                child.event(ctx, event, data, env);
            }
            Event::MouseUp(ev) if ev.buttons.has_left() => {
                self.is_lmb_pressed = false;

                child.event(ctx, event, data, env);
            }
            Event::MouseMove(_) if self.is_lmb_pressed => {
                let info = OnCustomSliderMovedInfo {
                    value: (*data) as u16,
                    custom_slider_id: self.custom_slider_id,
                };
                ctx.get_external_handle()
                    .submit_command(CUSTOM_SLIDER_ON_VALUE_CHANGED, info, Target::Auto)
                    .expect("failed to submit CUSTOM_SLIDER_ON_VALUE_CHANGED command");

                child.event(ctx, event, data, env);
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
