use iced::{
    button, checkbox, container, progress_bar, radio, rule, scrollable, slider, text_input, Color,
};

pub mod default_theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Default,
}

#[derive(Debug)]
pub struct StyleTheme {
    pub theme: Theme,
}

impl StyleTheme {
    pub fn new(theme: Theme) -> Self {
        StyleTheme { theme }
    }
    pub fn get_background_color(&self) -> Color {
        match self.theme {
            Theme::Default => default_theme::BACKGROUND_COLOR,
            // other themes ...
        }
    }
    pub fn get_message_author_color(&self) -> Color {
        match self.theme {
            Theme::Default => default_theme::MESSAGE_AUTHOR_COLOR,
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn container::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::Container.into(),
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn radio::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::Radio.into(),
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn text_input::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::TextInput.into(),
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn button::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::Button.into(),
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn scrollable::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::Scrollable.into(),
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn slider::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::Slider.into(),
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn progress_bar::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::ProgressBar.into(),
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn checkbox::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::Checkbox.into(),
            // other themes ...
        }
    }
}

impl From<Theme> for Box<dyn rule::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Default => default_theme::Rule.into(),
            // other themes ...
        }
    }
}
