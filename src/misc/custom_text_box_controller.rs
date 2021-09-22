use druid::widget::prelude::*;
use druid::widget::{Controller, TextBox, ValueTextBox};
use druid::Selector;
use druid_shell::keyboard_types::Key;

pub const CUSTOM_TEXT_BOX_RETURN_PRESSED: Selector =
    Selector::new("custom_text_box_return_pressed");

pub struct CustomTextBoxController {}

impl CustomTextBoxController {
    pub fn new() -> Self {
        CustomTextBoxController {}
    }
}

impl Controller<String, TextBox<String>> for CustomTextBoxController {
    fn event(
        &mut self,
        child: &mut TextBox<String>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        match event {
            Event::KeyUp(k) if k.key == Key::Enter && !k.mods.shift() => {
                ctx.submit_command(CUSTOM_TEXT_BOX_RETURN_PRESSED);
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}

impl Controller<String, ValueTextBox<String>> for CustomTextBoxController {
    fn event(
        &mut self,
        child: &mut ValueTextBox<String>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        match event {
            Event::KeyUp(k) if k.key == Key::Enter && !k.mods.shift() => {
                ctx.submit_command(CUSTOM_TEXT_BOX_RETURN_PRESSED);
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}
