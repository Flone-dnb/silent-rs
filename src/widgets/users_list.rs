use iced::{scrollable, Scrollable};

use crate::MainMessage;

#[derive(Debug, Default)]
pub struct UsersList {
    pub scroll_state: scrollable::State,
}

impl UsersList {
    pub fn get_ui(&mut self) -> Scrollable<MainMessage> {
        Scrollable::new(&mut self.scroll_state)
    }
}
