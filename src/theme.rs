use druid::widget::prelude::*;
use druid::{Color, Key, Lens};

pub const BACKGROUND_SPECIAL_COLOR: Key<Color> = Key::new("color.background_color_special");

#[derive(Clone, Data, Lens)]
pub struct ApplicationTheme {
    pub background_color: Color,
    pub background_special_color: Color,
    pub placeholder_color: Color,
    pub textbox_background_color: Color,
    pub text_selection_color: Color,
    pub active_border_color: Color,
    pub inactive_border_color: Color,
    pub button_dark_color: Color,
    pub button_light_color: Color,
    pub border_radius: f64,
}

impl Default for ApplicationTheme {
    fn default() -> Self {
        ApplicationTheme {
            background_color: Color::rgb8(30, 26, 22),
            background_special_color: Color::rgb8(35, 30, 25),
            placeholder_color: Color::rgb8(65, 60, 55),
            textbox_background_color: Color::rgb8(35, 30, 25),
            inactive_border_color: Color::rgba(0.0, 0.0, 0.0, 0.0),
            active_border_color: Color::rgb8(181, 98, 2),
            text_selection_color: Color::rgb8(181, 98, 2),
            button_dark_color: Color::rgb8(181, 98, 2),
            button_light_color: Color::rgb8(181, 98, 2),
            border_radius: 10.0,
        }
    }
}

impl ApplicationTheme {}
