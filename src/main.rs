#![feature(linked_list_remove)]
// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]
// External
use druid::widget::prelude::*;
use druid::widget::ViewSwitcher;
use druid::Lens;
use druid::{
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Handled, Target, WindowDesc,
};
use rdev::display_size;
use system_wide_key_state::*;

// Std
use std::sync::{Arc, Mutex};

// Custom.
mod global_params;
mod layouts;
mod misc;
mod services;
mod theme;
mod widgets;
use global_params::*;
use layouts::connect_layout::*;
use layouts::main_layout::*;
use layouts::settings_layout::*;
use misc::custom_data_button_controller::*;
use misc::custom_slider_controller::*;
use misc::custom_text_box_controller::*;
use services::audio_service::audio_service::*;
use services::config_service::*;
use services::net_service::*;
use services::user_tcp_service::*;
use services::user_udp_service::*;
use theme::*;

#[derive(PartialEq, Copy, Clone)]
pub enum CustomSliderID {
    MasterVolumeSlider,
    UserVolumeSlider,
}

#[derive(Clone, Copy, Data, PartialEq)]
pub enum Layout {
    Connect,
    Settings,
    Main,
}

#[derive(Clone, Data, Lens)]
pub struct ApplicationState {
    current_layout: Layout,
    connect_layout: ConnectLayout,
    settings_layout: SettingsLayout,
    main_layout: MainLayout,

    is_connected: bool,

    theme: ApplicationTheme,

    #[data(ignore)]
    audio_service: Arc<Mutex<AudioService>>,
    #[data(ignore)]
    network_service: Arc<Mutex<NetService>>,
    #[data(ignore)]
    user_config: Arc<Mutex<UserConfig>>,
}

pub fn main() {
    let window_size = Size {
        width: 650.0,
        height: 500.0,
    };

    let (w, h) = display_size().unwrap();

    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title("Silent")
        .window_size(window_size)
        .set_position((
            w as f64 / 2.0 - window_size.width / 2.0,
            h as f64 / 2.0 - window_size.height / 2.0,
        ));

    // load config
    let config = UserConfig::new();
    if let Err(err) = config {
        panic!("{} at [{}, {}]", err, file!(), line!());
    }

    // create the initial app state
    let mut initial_state: ApplicationState = ApplicationState {
        current_layout: Layout::Connect,
        connect_layout: ConnectLayout::new(),
        settings_layout: SettingsLayout::new(),
        main_layout: MainLayout::new(),
        theme: ApplicationTheme::default(),
        is_connected: false,
        audio_service: Arc::new(Mutex::new(AudioService::default())),
        network_service: Arc::new(Mutex::new(NetService::new())),
        user_config: Arc::new(Mutex::new(config.unwrap())),
    };

    init_app(&mut initial_state);

    // start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .delegate(Delegate {})
        .log_to_console()
        .configure_env(apply_theme)
        .launch(initial_state)
        .expect("Failed to launch the application.");
}

fn init_app(data: &mut ApplicationState) {
    let config_guard = data.user_config.lock().unwrap();

    // Fill connect fields from config.
    if let Err(msg) = data.connect_layout.read_user_config(&config_guard) {
        data.connect_layout // use connect result to show this error
            .set_connect_result(ConnectResult::Err(format!(
                "{} at [{}, {}]",
                msg,
                file!(),
                line!()
            )));
    }

    //data.settings_layout.ui_scaling_slider_value = config.ui_scaling as i32;
    //data.ui_scaling = config.ui_scaling as f64 / 100.0;
    data.settings_layout.master_volume = config_guard.master_volume as f64;
    data.settings_layout.push_to_talk_key_text = get_key_name(config_guard.push_to_talk_button);
    data.settings_layout.push_to_talk_keycode = config_guard.push_to_talk_button;

    data.audio_service.lock().unwrap().init(
        Arc::clone(&data.network_service),
        config_guard.master_volume as i32,
    );
}

struct Delegate;

impl AppDelegate<ApplicationState> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut ApplicationState,
        _env: &Env,
    ) -> Handled {
        if let Some(key) = cmd.get(PUSH_TO_TALK_KEY_CHANGE_EVENT) {
            SettingsLayout::push_to_talk_key_change_event(data, key);
            Handled::Yes
        } else if cmd.get(CUSTOM_TEXT_BOX_RETURN_PRESSED).is_some() {
            MainLayout::send_message_event(data);
            Handled::Yes
        } else if let Some(button_info) = cmd.get(CUSTOM_DATA_BUTTON_CLICKED) {
            MainLayout::connect_list_item_pressed_event(data, button_info);
            Handled::Yes
        } else if let Some(info) = cmd.get(CUSTOM_SLIDER_ON_VALUE_CHANGED) {
            match info.custom_slider_id {
                CustomSliderID::MasterVolumeSlider => {
                    SettingsLayout::master_volume_slider_moved_event(data, info);
                }
                CustomSliderID::UserVolumeSlider => {
                    MainLayout::user_volume_slider_moved_event(data);
                }
            }
            Handled::Yes
        } else if let Some(username) = cmd.get(AUDIO_SERVICE_ON_USER_TALK_START) {
            data.main_layout.set_user_talking(username, true);
            Handled::Yes
        } else if let Some(username) = cmd.get(AUDIO_SERVICE_ON_USER_TALK_END) {
            data.main_layout.set_user_talking(username, false);
            Handled::Yes
        } else if let Some(error_msg) = cmd.get(NETWORK_SERVICE_SYSTEM_IO_ERROR) {
            data.main_layout.add_system_message(error_msg.clone());
            Handled::Yes
        } else if let Some(count) = cmd.get(NETWORK_SERVICE_UPDATE_CONNECTED_USERS_COUNT) {
            data.main_layout.connected_count_text = *count;
            Handled::Yes
        } else if let Some(ping_data) = cmd.get(USER_UDP_SERVICE_UPDATE_USER_PING) {
            if let Err(_) = data
                .main_layout
                .set_user_ping(&ping_data.username, ping_data.ping_ms)
            {
                if ping_data.try_again_count == 0 {
                    data.main_layout.add_system_message(format!(
                                    "Ping of user '{}' was received but no info about the user was received (ping of unknown user) [failed after {} attempts to wait for user info].",
                                    &ping_data.username,
                                    USER_CONNECT_FIRST_UDP_PING_RETRY_MAX_COUNT
                                ));
                } else {
                    data.network_service.lock().unwrap().resend_ping_later(ping_data.clone());
                }
            }
            Handled::Yes
        } else if let Some(username) = cmd.get(USER_TCP_SERVICE_USER_CONNECTED) {
            {
                let audio_guard = data.audio_service.lock().unwrap();

                let mut users_audio_data_guard = audio_guard.users_voice_data.lock().unwrap();

                users_audio_data_guard
                    .push(Arc::new(Mutex::new(UserVoiceData::new(username.clone()))));
            }
            if let Err(msg) =
                data.main_layout
                    .add_user(username.clone(), String::from(""), 0, false)
            {
                data.main_layout.add_system_message(format!(
                    "{} at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            Handled::Yes
        } else if let Some(username) = cmd.get(USER_TCP_SERVICE_USER_DISCONNECTED) {
            {
                let audio_guard = data.audio_service.lock().unwrap();

                let mut users_audio_data_guard = audio_guard.users_voice_data.lock().unwrap();

                let mut found = false;
                let mut found_i = 0usize;

                for (i, user) in users_audio_data_guard.iter().enumerate() {
                    let user_guard = user.lock().unwrap();
                    if &user_guard.username == username {
                        found = true;
                        found_i = i;
                        break;
                    }
                }

                if found {
                    users_audio_data_guard.remove(found_i);
                } else {
                    data.main_layout.add_system_message(format!(
                        "Warning: can't find user ('{}') at [{}, {}]",
                        username,
                        file!(),
                        line!()
                    ));
                }
            }

            if let Err(msg) = data.main_layout.remove_user(username) {
                data.main_layout.add_system_message(msg);
            }
            Handled::Yes
        } else if cmd.get(NETWORK_SERVICE_CLEAR_ALL_USERS).is_some() {
            data.main_layout.clear_all_users();
            Handled::Yes
        } else if let Some(user_message_info) = cmd.get(USER_TCP_SERVICE_USER_MESSAGE) {
            data.main_layout.add_message(
                user_message_info.message.clone(),
                user_message_info.username.clone(),
            );
            Handled::Yes
        } else if let Some(user_message_info) = cmd.get(USER_TCP_SERVICE_MOVE_USER_TO_ROOM) {
            if let Err(msg) = data
                .main_layout
                .move_user(&user_message_info.username, &user_message_info.room_to)
            {
                data.main_layout.add_system_message(msg);
            } else {
                if user_message_info.username == data.main_layout.current_user_name {
                    data.main_layout.clear_text_chat();
                    data.main_layout.current_user_room = user_message_info.room_to.clone();
                }
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

fn apply_theme(env: &mut Env, data: &ApplicationState) {
    env.set(
        druid::theme::WINDOW_BACKGROUND_COLOR,
        data.theme.background_color.clone(),
    );
    env.set(
        druid::theme::TEXTBOX_BORDER_RADIUS,
        data.theme.border_radius,
    );
    env.set(
        druid::theme::BUTTON_BORDER_RADIUS,
        data.theme.border_radius,
    );
    env.set(
        druid::theme::PLACEHOLDER_COLOR,
        data.theme.placeholder_color.clone(),
    );
    env.set(
        druid::theme::BACKGROUND_LIGHT,
        data.theme.textbox_background_color.clone(),
    );
    env.set(
        druid::theme::BORDER_DARK,
        data.theme.inactive_border_color.clone(),
    );
    env.set(
        druid::theme::SELECTED_TEXT_BACKGROUND_COLOR,
        data.theme.text_selection_color.clone(),
    );
    env.set(
        druid::theme::PRIMARY_LIGHT,
        data.theme.active_border_color.clone(),
    );
    env.set(
        druid::theme::BUTTON_DARK,
        data.theme.button_dark_color.clone(),
    );
    env.set(
        druid::theme::BUTTON_LIGHT,
        data.theme.button_light_color.clone(),
    );

    env.set(
        BACKGROUND_SPECIAL_COLOR,
        data.theme.background_special_color.clone(),
    );
}

fn build_root_widget() -> impl Widget<ApplicationState> {
    ViewSwitcher::new(
        |data: &ApplicationState, _env| data.current_layout,
        |selector, _data, _env| match *selector {
            Layout::Connect => Box::new(ConnectLayout::build_ui()),
            Layout::Settings => Box::new(SettingsLayout::build_ui()),
            Layout::Main => Box::new(MainLayout::build_ui()),
        },
    )
}
