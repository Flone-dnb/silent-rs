use iced::{
    executor, text_input, window::icon::Icon, Align, Application, Clipboard, Color, Column,
    Command, Element, HorizontalAlignment, Length, Row, Settings, Text, TextInput,
};

use std::fs::File;

mod global_params;
mod themes;
mod widgets;
use global_params::*;
use themes::StyleTheme;
use themes::Theme;
use widgets::chat_list::ChatList;
use widgets::users_list::UsersList;

fn read_icon_png(path: String) -> Vec<u8> {
    let decoder = png::Decoder::new(File::open(path).unwrap());
    let (info, mut reader) = decoder.read_info().unwrap();
    let mut buf = vec![0; info.buffer_size()];

    reader.next_frame(&mut buf).unwrap();

    buf
}

fn main() -> iced::Result {
    let mut config = Settings::default();
    config.antialiasing = false;
    config.window.size = (1200, 600);
    config.window.min_size = Some((800, 500));
    config.default_font = Some(include_bytes!("../res/mplus-2p-light.ttf"));

    let icon = Icon::from_rgba(read_icon_png(String::from("res/app_icon.png")), 256, 256).unwrap();
    config.window.icon = Some(icon);
    config.default_text_size = 30;
    Silent::run(config)
}

#[derive(Debug)]
struct Silent {
    chat_list: ChatList,
    users_list: UsersList,
    message_string: String,

    style: StyleTheme,

    message_input: text_input::State,
}

#[derive(Debug, Clone)]
pub enum MainMessage {
    MessageInputChanged(String),
}

impl Silent {
    fn new() -> Self {
        Silent {
            chat_list: ChatList::new(),
            users_list: UsersList::default(),
            message_string: String::default(),
            message_input: text_input::State::default(),
            style: StyleTheme::new(Theme::Default),
        }
    }
}

impl Application for Silent {
    type Executor = executor::Default;
    type Message = MainMessage;
    type Flags = ();

    fn new(_flags: ()) -> (Silent, Command<MainMessage>) {
        (Silent::new(), Command::none())
    }

    fn background_color(&self) -> Color {
        self.style.get_background_color()
    }

    fn title(&self) -> String {
        String::from("Silent")
    }

    fn update(
        &mut self,
        message: Self::Message,
        clipboard: &mut Clipboard,
    ) -> Command<Self::Message> {
        match message {
            MainMessage::MessageInputChanged(text) => {
                if text.chars().count() <= MAX_MESSAGE_SIZE {
                    self.message_string = text
                }
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<MainMessage> {
        self.chat_list.add_message(
            String::from("Привет мир! Hello World!"),
            String::from("Flone"),
        );

        self.chat_list.add_message(
            String::from("Привет мир! Hello World!"),
            String::from("Foo"),
        );

        self.chat_list
            .add_message(String::from("Addition string!"), String::from("Foo"));

        self.users_list.add_user(String::from("Flone"));
        self.users_list.add_user(String::from("Foo"));

        let left: Column<MainMessage> = Column::new()
            .align_items(Align::Center)
            .padding(10)
            .spacing(10)
            .push(
                Text::new("Text Chat")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .color(Color::WHITE)
                    .height(Length::FillPortion(15)),
            )
            .push(
                self.chat_list
                    .get_ui()
                    .height(Length::FillPortion(83))
                    .style(self.style.theme),
            )
            .push(
                Row::new()
                    .push(
                        TextInput::new(
                            &mut self.message_input,
                            "Type your message here...",
                            &self.message_string,
                            MainMessage::MessageInputChanged,
                        )
                        .size(22)
                        .style(self.style.theme),
                    )
                    .height(Length::FillPortion(7)),
            );

        let right: Column<MainMessage> = Column::new()
            .align_items(Align::Center)
            .padding(10)
            .spacing(10)
            .push(
                Text::new("Connected: 0")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .color(Color::WHITE)
                    .height(Length::FillPortion(15)),
            )
            .push(
                self.users_list
                    .get_ui()
                    .height(Length::FillPortion(85))
                    .style(self.style.theme),
            );

        Row::new()
            .padding(10)
            .align_items(Align::Center)
            .push(left.width(Length::FillPortion(60)))
            .push(right.width(Length::FillPortion(40)))
            .into()
    }
}
