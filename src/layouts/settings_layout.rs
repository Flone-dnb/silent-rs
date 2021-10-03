// External.
use druid::widget::prelude::*;
use druid::widget::{
    Button, Container, CrossAxisAlignment, Flex, Label, LineBreaking, MainAxisAlignment, Padding,
    Scroll, SizedBox, Slider, ViewSwitcher,
};
use druid::{Color, Data, Lens, LensExt, Selector, Target, WidgetExt};
use rdev::{listen, EventType};
use system_wide_key_state::*;

// Std.
use std::thread;

// Custom.
use crate::misc::{custom_slider_controller::*, locale_keys::*};
use crate::services::user_tcp_service::ConnectResult;
use crate::theme::*;
use crate::ApplicationState;
use crate::CustomSliderID;
use crate::{global_params::*, Layout};

pub const PUSH_TO_TALK_KEY_CHANGE_EVENT: Selector<String> =
    Selector::new("settings_push_to_talk_key_change_event");

#[derive(Data, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum SupportedLocale {
    En,
    Ru,
}

#[derive(Clone, Data, PartialEq)]
pub enum ActiveOption {
    General,
    About,
}

#[derive(Clone, Data, Lens)]
pub struct SettingsLayout {
    pub active_option: ActiveOption,
    pub show_message_notification: bool,
    pub master_volume: f64,
    pub push_to_talk_key_text: String,
    pub selected_locale: SupportedLocale,
    #[data(ignore)]
    pub push_to_talk_keycode: KeyCode,
}

impl SettingsLayout {
    pub fn new() -> Self {
        SettingsLayout {
            active_option: ActiveOption::General,
            master_volume: 100.0,
            push_to_talk_key_text: "T".to_string(),
            push_to_talk_keycode: KeyCode::KT,
            show_message_notification: true,
            selected_locale: SupportedLocale::En,
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        let mut active_option_content = Flex::column()
            .must_fill_main_axis(true)
            .main_axis_alignment(MainAxisAlignment::Center)
            .with_flex_child(SizedBox::empty().expand(), 10.0);

        let res = ViewSwitcher::new(
            |data: &ApplicationState, _env| data.settings_layout.active_option.clone(),
            |selector, _data, _env| match selector {
                &ActiveOption::General => Box::new(
                    Flex::column()
                        .must_fill_main_axis(true)
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .with_flex_child(
                            Container::new(SizedBox::empty().expand())
                                .background(BACKGROUND_SPECIAL_COLOR)
                                .expand(),
                            10.0,
                        )
                        .with_flex_child(SizedBox::empty().expand(), 5.0)
                        .with_flex_child(Container::new(SizedBox::empty().expand()).expand(), 10.0)
                        .expand(),
                ),
                &ActiveOption::About => Box::new(
                    Flex::column()
                        .must_fill_main_axis(true)
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .with_flex_child(Container::new(SizedBox::empty().expand()).expand(), 10.0)
                        .with_flex_child(SizedBox::empty().expand(), 5.0)
                        .with_flex_child(
                            Container::new(SizedBox::empty().expand())
                                .background(BACKGROUND_SPECIAL_COLOR)
                                .expand(),
                            10.0,
                        )
                        .expand(),
                ),
            },
        );

        active_option_content.add_flex_child(res, 25.0);
        active_option_content.add_flex_child(SizedBox::empty().expand(), 45.0);
        // for back button
        active_option_content.add_flex_child(SizedBox::empty().expand(), 10.0);
        active_option_content.add_flex_child(SizedBox::empty().expand(), 10.0);

        Flex::row()
            .with_flex_child(SizedBox::empty().expand(), 5.0)
            .with_flex_child(
                Container::new(
                    Flex::column()
                        .must_fill_main_axis(true)
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .with_flex_child(SizedBox::empty().expand(), 10.0)
                        .with_flex_child(
                            Button::from_label(
                                Label::new(|data: &ApplicationState, _env: &Env| {
                                    data.localization
                                        .get(LOCALE_SETTINGS_LAYOUT_GENERAL_SECTION_TEXT)
                                        .unwrap()
                                        .clone()
                                })
                                .with_text_size(TEXT_SIZE),
                            )
                            .on_click(SettingsLayout::on_general_button_clicked)
                            .expand(),
                            10.0,
                        )
                        .with_flex_child(SizedBox::empty().expand(), 5.0)
                        .with_flex_child(
                            Button::from_label(
                                Label::new(|data: &ApplicationState, _env: &Env| {
                                    data.localization
                                        .get(LOCALE_SETTINGS_LAYOUT_ABOUT_SECTION_TEXT)
                                        .unwrap()
                                        .clone()
                                })
                                .with_text_size(TEXT_SIZE),
                            )
                            .on_click(SettingsLayout::on_about_button_clicked)
                            .expand(),
                            10.0,
                        )
                        .with_flex_child(SizedBox::empty().expand(), 45.0)
                        .with_flex_child(
                            Button::from_label(
                                Label::new(|data: &ApplicationState, _env: &Env| {
                                    data.localization
                                        .get(LOCALE_SETTINGS_LAYOUT_BACK_BUTTON_TEXT)
                                        .unwrap()
                                        .clone()
                                })
                                .with_text_size(TEXT_SIZE),
                            )
                            .on_click(SettingsLayout::on_back_button_clicked)
                            .expand(),
                            10.0,
                        )
                        .with_flex_child(SizedBox::empty().expand(), 10.0),
                )
                .background(BACKGROUND_SPECIAL_COLOR)
                .expand(),
                20.0,
            )
            .with_flex_child(active_option_content, 5.0)
            .with_flex_child(
                Flex::column()
                    .must_fill_main_axis(true)
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .with_flex_child(SizedBox::empty().expand(), 5.0)
                    .with_flex_child(
                        Container::new(ViewSwitcher::new(
                            |data: &ApplicationState, _env| {
                                data.settings_layout.active_option.clone()
                            },
                            |selector, _data, _env| match selector {
                                &ActiveOption::General => {
                                    Box::new(SettingsLayout::get_general_content())
                                }
                                &ActiveOption::About => {
                                    Box::new(SettingsLayout::get_about_content())
                                }
                            },
                        ))
                        .background(BACKGROUND_SPECIAL_COLOR)
                        .rounded(druid::theme::BUTTON_BORDER_RADIUS)
                        .expand(),
                        90.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), 5.0),
                65.0,
            )
            .with_flex_child(SizedBox::empty().expand(), 5.0)
    }
    fn on_general_button_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.settings_layout.active_option = ActiveOption::General;
    }
    fn on_about_button_clicked(ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        // finish changing push-to-talk button if it was pressed
        ctx.get_external_handle()
            .submit_command(PUSH_TO_TALK_KEY_CHANGE_EVENT, String::new(), Target::Auto)
            .expect("failed to submit PUSH_TO_TALK_KEY_CHANGE_EVENT command");

        data.settings_layout.active_option = ActiveOption::About;
    }
    fn on_locale_changed(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        // Save to config.
        let mut config_guard = data.user_config.lock().unwrap();
        match data.settings_layout.selected_locale {
            SupportedLocale::En => config_guard.locale = String::from("en"),
            SupportedLocale::Ru => config_guard.locale = String::from("ru"),
        }

        if let Err(err) = config_guard.save() {
            let error_msg = format!("{} at [{}, {}]", err, file!(), line!());
            if !data.is_connected {
                data.connect_layout
                    .set_connect_result(ConnectResult::Err(error_msg));
            } else {
                data.main_layout.add_system_message(error_msg);
            }
        }
    }
    fn on_back_button_clicked(ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        // finish changing push-to-talk button if it was pressed
        ctx.get_external_handle()
            .submit_command(PUSH_TO_TALK_KEY_CHANGE_EVENT, String::new(), Target::Auto)
            .expect("failed to submit PUSH_TO_TALK_KEY_CHANGE_EVENT command");

        if data.is_connected {
            data.current_layout = Layout::Main;
        } else {
            data.current_layout = Layout::Connect;
        }
    }
    fn on_show_message_notification_clicked(
        _ctx: &mut EventCtx,
        data: &mut ApplicationState,
        _env: &Env,
    ) {
        data.settings_layout.show_message_notification =
            !data.settings_layout.show_message_notification;

        // Save to config.
        let mut config_guard = data.user_config.lock().unwrap();
        config_guard.show_message_notification = data.settings_layout.show_message_notification;

        if let Err(err) = config_guard.save() {
            let error_msg = format!("{} at [{}, {}]", err, file!(), line!());
            if !data.is_connected {
                data.connect_layout
                    .set_connect_result(ConnectResult::Err(error_msg));
            } else {
                data.main_layout.add_system_message(error_msg);
            }
        }
    }
    fn on_push_to_talk_clicked(ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.settings_layout.push_to_talk_key_text = data
            .localization
            .get(LOCALE_SETTINGS_LAYOUT_SETTING_PUSH_TO_TALK_BUTTON_CHANGE_TEXT)
            .unwrap()
            .clone();

        let event_sink = ctx.get_external_handle();
        thread::spawn(move || {
            // Listen to keyboard/mouse.
            // this will block forever (this is why we need a restart)
            if let Err(error) = listen(move |event| {
                if let EventType::KeyPress(key) = event.event_type {
                    // convert key to our enum
                    let mut _key_name: String = String::new();
                    loop {
                        if let Some(keycode) = convert_rdev_key(key) {
                            _key_name = get_key_name(keycode);
                            break;
                        }
                    }

                    if _key_name == get_key_name(KeyCode::KEsc) {
                        _key_name = String::new(); // don't update, but hide "Press any key" text
                    }

                    event_sink
                        .submit_command(PUSH_TO_TALK_KEY_CHANGE_EVENT, _key_name, Target::Auto)
                        .expect("failed to submit PUSH_TO_TALK_KEY_CHANGE_EVENT command");
                }
            }) {
                println!("rdev listen error: {:?}", error);
            }
        });
    }
    fn get_general_content() -> impl Widget<ApplicationState> {
        Padding::new(
            10.0,
            Flex::column()
                .must_fill_main_axis(true)
                .main_axis_alignment(MainAxisAlignment::Start)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(
                    Label::new(|data: &ApplicationState, _env: &Env| {
                        data.localization
                            .get(LOCALE_SETTINGS_LAYOUT_RESTART_NOTE_TEXT)
                            .unwrap()
                            .clone()
                    })
                    .with_text_color(Color::RED)
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(TEXT_SIZE),
                )
                .with_default_spacer()
                .with_default_spacer()
                .with_child(
                    Label::new(|data: &ApplicationState, _env: &Env| {
                        data.localization
                            .get(LOCALE_SETTINGS_LAYOUT_SETTING_MASTER_VOLUME_TEXT)
                            .unwrap()
                            .clone()
                    })
                    .with_text_size(TEXT_SIZE),
                )
                .with_child(
                    Flex::row()
                        .must_fill_main_axis(true)
                        .with_flex_child(
                            Slider::new()
                                .with_step(1.0)
                                .with_range(0.0, 100.0)
                                .expand_width()
                                .controller(CustomSliderController::new(
                                    CustomSliderID::MasterVolumeSlider,
                                ))
                                .lens(
                                    ApplicationState::settings_layout
                                        .then(SettingsLayout::master_volume),
                                ),
                            80.0,
                        )
                        .with_flex_child(
                            Label::new(|data: &ApplicationState, _env: &Env| {
                                format!("{:.3} %", data.settings_layout.master_volume.to_string())
                            })
                            .with_text_size(TEXT_SIZE),
                            20.0,
                        ),
                )
                .with_default_spacer()
                .with_child(
                    Flex::row()
                        .with_child(
                            Label::new(|data: &ApplicationState, _env: &Env| {
                                format!(
                                    "{}:  ",
                                    data.localization
                                        .get(
                                            LOCALE_SETTINGS_LAYOUT_SETTING_PUSH_TO_TALK_BUTTON_TEXT
                                        )
                                        .unwrap()
                                )
                            })
                            .with_text_size(TEXT_SIZE),
                        )
                        .with_child(
                            Button::from_label(
                                Label::new(|data: &ApplicationState, _env: &Env| {
                                    data.settings_layout.push_to_talk_key_text.clone()
                                })
                                .with_text_size(TEXT_SIZE),
                            )
                            .on_click(SettingsLayout::on_push_to_talk_clicked),
                        ),
                )
                .with_default_spacer()
                .with_child(
                    Flex::row()
                        .with_child(Label::new(|data: &ApplicationState, _env: &Env| {
                                format!(
                                    "{}:  ",
                                    data.localization
                                        .get(
                                            LOCALE_SETTINGS_LAYOUT_SETTING_MESSAGE_NOTIFICATIONS_TEXT
                                        )
                                        .unwrap()
                                )
                            }).with_text_size(TEXT_SIZE))
                        .with_child(
                            Button::from_label(
                                Label::new(|data: &ApplicationState, _env: &Env| {
                                    if data.settings_layout.show_message_notification {
                                        data.localization
                                        .get(
                                            LOCALE_SETTINGS_LAYOUT_SETTING_MESSAGE_NOTIFICATIONS_BUTTON_ON_TEXT
                                        )
                                        .unwrap().clone()
                                    } else {
                                        data.localization
                                        .get(
                                            LOCALE_SETTINGS_LAYOUT_SETTING_MESSAGE_NOTIFICATIONS_BUTTON_OFF_TEXT
                                        )
                                        .unwrap().clone()
                                    }
                                })
                                .with_text_size(TEXT_SIZE),
                            )
                            .on_click(SettingsLayout::on_show_message_notification_clicked),
                        ),
                )
                .with_default_spacer()
                .with_child(
                    Flex::column()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .with_child(
                            Label::new(|data: &ApplicationState, _env: &Env| {
                                let current_language = match data.settings_layout.selected_locale{
                                    SupportedLocale::En => "English",
                                    SupportedLocale::Ru => "Русский",
                                };
                                format!("{}: {}", 
                                    data.localization.get(LOCALE_SETTINGS_LAYOUT_SETTING_LOCALE_TEXT)
                                    .unwrap(), current_language
                                )
                            })
                            .with_text_size(TEXT_SIZE),
                        )
                        .with_child(
                            Scroll::new(
                                Flex::column()
                                    .cross_axis_alignment(CrossAxisAlignment::Start)
                                    .with_child(
                                        Button::from_label(Label::new("English").with_text_size(TEXT_SIZE))
                                            .expand_width()
                                            .on_click(|ctx: &mut EventCtx, data: &mut ApplicationState, env: &Env|{
                                                data.settings_layout.selected_locale = SupportedLocale::En;
                                                SettingsLayout::on_locale_changed(ctx, data, env);
                                            })
                                    )
                                    .with_child(
                                        Button::from_label(Label::new("Русский").with_text_size(TEXT_SIZE))
                                            .expand_width()
                                            .on_click(|ctx: &mut EventCtx, data: &mut ApplicationState, env: &Env|{
                                                data.settings_layout.selected_locale = SupportedLocale::Ru;
                                                SettingsLayout::on_locale_changed(ctx, data, env);
                                            })
                                    )
                                ).vertical()
                        )
                ),
        )
    }
    fn get_about_content() -> impl Widget<ApplicationState> {
        Padding::new(
            10.0,
            Flex::column()
                .must_fill_main_axis(true)
                .main_axis_alignment(MainAxisAlignment::Start)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(
                    Label::new(|data: &ApplicationState, _env: &Env| {
                        format!(
                            "{}\n",
                            data.localization
                                .get(LOCALE_SETTINGS_LAYOUT_ABOUT_SECTION_APP_DESC_TEXT)
                                .unwrap()
                        )
                    })
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(TEXT_SIZE),
                )
                .with_child(
                    Label::new(|data: &ApplicationState, _env: &Env| {
                        format!(
                            "{}: {} (rs).",
                            data.localization
                                .get(LOCALE_SETTINGS_LAYOUT_ABOUT_SECTION_APP_VERSION_TEXT)
                                .unwrap(),
                            env!("CARGO_PKG_VERSION"),
                        )
                    })
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(TEXT_SIZE),
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Label::new(|data: &ApplicationState, _env: &Env| {
                                format!(
                            "{} ",
                            data.localization
                                .get(LOCALE_SETTINGS_LAYOUT_ABOUT_SECTION_APP_SOURCE_CODE_TEXT)
                                .unwrap())
                            })
                            .with_line_break_mode(LineBreaking::WordWrap)
                            .with_text_size(TEXT_SIZE),
                        )
                        .with_child(
                            Button::from_label(Label::new(|data: &ApplicationState, _env: &Env| {
                                format!(
                            "{}",
                            data.localization
                                .get(LOCALE_SETTINGS_LAYOUT_ABOUT_SECTION_APP_SOURCE_CODE_BUTTON_TEXT)
                                .unwrap())
                            }).with_text_size(TEXT_SIZE))
                                .on_click(|_ctx, _data, _env| {
                                    opener::open("https://github.com/Flone-dnb/silent-rs").unwrap();
                                }),
                        ),
                )
                .with_child(
                    Label::new(
                        |data: &ApplicationState, _env: &Env| {
                                format!(
                            "\n{}",
                            data.localization
                                .get(LOCALE_SETTINGS_LAYOUT_ABOUT_SECTION_DRUID_TEXT)
                                .unwrap())
                            }
                    )
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(TEXT_SIZE),
                ),
        )
    }
    pub fn push_to_talk_key_change_event(data: &mut ApplicationState, key: &String) {
        if key == "" {
            data.settings_layout.push_to_talk_key_text =
                get_key_name(data.settings_layout.push_to_talk_keycode);
        } else {
            data.settings_layout.push_to_talk_key_text = key.to_string();
            data.settings_layout.push_to_talk_keycode = string_to_key(key);

            // Save to config.
            let mut config_guard = data.user_config.lock().unwrap();
            config_guard.push_to_talk_button = string_to_key(key);

            if let Err(err) = config_guard.save() {
                let error_msg = format!("{} at [{}, {}]", err, file!(), line!());
                if !data.is_connected {
                    data.connect_layout
                        .set_connect_result(ConnectResult::Err(error_msg));
                } else {
                    data.main_layout.add_system_message(error_msg);
                }
            }
        }
    }
    pub fn master_volume_slider_moved_event(
        data: &mut ApplicationState,
        info: &OnCustomSliderMovedInfo,
    ) {
        // Save to config.
        let mut config_guard = data.user_config.lock().unwrap();
        config_guard.master_volume = info.value;

        if let Err(err) = config_guard.save() {
            let error_msg = format!("{} at [{}, {}]", err, file!(), line!());
            if !data.is_connected {
                data.connect_layout
                    .set_connect_result(ConnectResult::Err(error_msg));
            } else {
                data.main_layout.add_system_message(error_msg);
            }
        }
    }
}

fn convert_rdev_key(key: rdev::Key) -> Option<system_wide_key_state::KeyCode> {
    match key {
        // only use some of the keys that will most likely be used
        rdev::Key::Tab => Some(KeyCode::KTab),
        rdev::Key::ShiftLeft => Some(KeyCode::KShift),
        rdev::Key::ControlLeft => Some(KeyCode::KCtrl),
        rdev::Key::Alt => Some(KeyCode::KAlt),
        rdev::Key::CapsLock => Some(KeyCode::KCapsLock),
        rdev::Key::Escape => Some(KeyCode::KEsc), // cancels change
        rdev::Key::Space => Some(KeyCode::KSpace),
        rdev::Key::Kp0 => Some(KeyCode::K0),
        rdev::Key::Kp1 => Some(KeyCode::K1),
        rdev::Key::Kp2 => Some(KeyCode::K2),
        rdev::Key::Kp3 => Some(KeyCode::K3),
        rdev::Key::Kp4 => Some(KeyCode::K4),
        rdev::Key::Kp5 => Some(KeyCode::K5),
        rdev::Key::Kp6 => Some(KeyCode::K6),
        rdev::Key::Kp7 => Some(KeyCode::K7),
        rdev::Key::Kp8 => Some(KeyCode::K8),
        rdev::Key::Kp9 => Some(KeyCode::K9),
        rdev::Key::KeyA => Some(KeyCode::KA),
        rdev::Key::KeyB => Some(KeyCode::KB),
        rdev::Key::KeyC => Some(KeyCode::KC),
        rdev::Key::KeyD => Some(KeyCode::KD),
        rdev::Key::KeyE => Some(KeyCode::KE),
        rdev::Key::KeyF => Some(KeyCode::KF),
        rdev::Key::KeyG => Some(KeyCode::KG),
        rdev::Key::KeyH => Some(KeyCode::KH),
        rdev::Key::KeyI => Some(KeyCode::KI),
        rdev::Key::KeyJ => Some(KeyCode::KJ),
        rdev::Key::KeyK => Some(KeyCode::KK),
        rdev::Key::KeyL => Some(KeyCode::KL),
        rdev::Key::KeyM => Some(KeyCode::KM),
        rdev::Key::KeyN => Some(KeyCode::KN),
        rdev::Key::KeyO => Some(KeyCode::KO),
        rdev::Key::KeyP => Some(KeyCode::KP),
        rdev::Key::KeyQ => Some(KeyCode::KQ),
        rdev::Key::KeyR => Some(KeyCode::KR),
        rdev::Key::KeyS => Some(KeyCode::KS),
        rdev::Key::KeyT => Some(KeyCode::KT),
        rdev::Key::KeyU => Some(KeyCode::KU),
        rdev::Key::KeyV => Some(KeyCode::KV),
        rdev::Key::KeyW => Some(KeyCode::KW),
        rdev::Key::KeyX => Some(KeyCode::KX),
        rdev::Key::KeyY => Some(KeyCode::KY),
        rdev::Key::KeyZ => Some(KeyCode::KZ),
        _ => None,
    }
}
