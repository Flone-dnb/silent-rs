// External.
use chrono::prelude::*;
use druid::{ExtEventSink, Selector, Target};
use system_wide_key_state::*;

// Std.
use std::convert::TryInto;
use std::net::*;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use super::tcp_packets::*;
use crate::global_params::*;
use crate::services::audio_service::audio_service::*;
use crate::services::user_tcp_service::*;
use crate::services::user_udp_service::*;

pub const NETWORK_SERVICE_SYSTEM_IO_ERROR: Selector<String> =
    Selector::new("network_service_system_io_error");

pub const NETWORK_SERVICE_UPDATE_CONNECTED_USERS_COUNT: Selector<usize> =
    Selector::new("network_service_update_connected_users_count");

pub const NETWORK_SERVICE_CLEAR_ALL_USERS: Selector<()> =
    Selector::new("network_service_clear_all_users");

pub enum ActionError {
    ChangeRoomsTooQuick,
    SendMessagesTooQuick,
    SystemError(String),
}

pub struct ClientConfig {
    pub username: String,
    pub server_name: String,
    pub server_port: String,
    pub server_password: String,
    pub push_to_talk_key: KeyCode,
}

#[derive(Clone)] // for ApplicationState
pub struct PasswordRetrySleep {
    pub sleep_time_start: DateTime<Local>,
    pub sleep_time_sec: usize,
    pub sleep: bool,
}

#[derive(Clone)] // for ApplicationState
pub struct NetService {
    pub user_tcp_service: Arc<Mutex<UserTcpService>>,
    pub user_udp_service: Arc<Mutex<UserUdpService>>,
    pub audio_service: Option<Arc<Mutex<AudioService>>>,
    pub password_retry: PasswordRetrySleep,
    pub event_sink: Option<ExtEventSink>,
    last_time_text_message_sent: DateTime<Local>,
    last_time_entered_room: DateTime<Local>,
}

impl NetService {
    pub fn new() -> Self {
        Self {
            user_tcp_service: Arc::new(Mutex::new(UserTcpService::new(String::from("")))),
            user_udp_service: Arc::new(Mutex::new(UserUdpService::new())),
            last_time_text_message_sent: Local::now(),
            last_time_entered_room: Local::now(),
            audio_service: None,
            password_retry: PasswordRetrySleep {
                sleep_time_start: Local::now(),
                sleep_time_sec: 0,
                sleep: false,
            },
            event_sink: None,
        }
    }

    pub fn init_audio_service(&mut self, audio_service: Arc<Mutex<AudioService>>) {
        self.audio_service = Some(audio_service);
    }

    pub fn resend_ping_later(&self, ping_data: UserPingInfo) {
        let event_sink_clone = self.event_sink.clone().unwrap();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(
                USER_CONNECT_FIRST_UDP_PING_RETRY_INTERVAL_MS as u64,
            ));
            event_sink_clone
                .submit_command(
                    USER_UDP_SERVICE_UPDATE_USER_PING,
                    UserPingInfo {
                        username: ping_data.username,
                        ping_ms: ping_data.ping_ms,
                        try_again_count: ping_data.try_again_count - 1,
                    },
                    Target::Auto,
                )
                .expect("failed to submit USER_UDP_SERVICE_UPDATE_USER_PING command");
        });
    }

    pub fn start(
        &mut self,
        config: ClientConfig,
        username: String,
        server_password: String,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
        event_sink: ExtEventSink,
    ) {
        self.event_sink = Some(event_sink.clone());

        if self.password_retry.sleep {
            let time_diff = Local::now() - self.password_retry.sleep_time_start;
            if time_diff.num_seconds() < self.password_retry.sleep_time_sec as i64 {
                return;
            }
        }

        // Start TCP service.
        self.user_tcp_service = Arc::new(Mutex::new(UserTcpService::new(server_password)));
        let user_tcp_service_copy = Arc::clone(&self.user_tcp_service);
        let user_udp_service_copy = Arc::clone(&self.user_udp_service);
        let audio_service_copy = Arc::clone(self.audio_service.as_ref().unwrap());
        thread::spawn(move || {
            NetService::tcp_service(
                config,
                username,
                user_tcp_service_copy,
                user_udp_service_copy,
                connect_layout_sender,
                event_sink,
                audio_service_copy,
            )
        });
    }
    pub fn enter_room(&mut self, room: &str) -> Result<(), ActionError> {
        let time_diff = Local::now() - self.last_time_entered_room;
        if time_diff.num_seconds() < SPAM_PROTECTION_SEC as i64 {
            return Err(ActionError::ChangeRoomsTooQuick);
        }

        match self.user_tcp_service.lock().unwrap().enter_room(room) {
            HandleMessageResult::Ok => {}
            HandleMessageResult::IOError(err) => match err {
                IoResult::Err(msg) => {
                    return Err(ActionError::SystemError(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                _ => {}
            },
            HandleMessageResult::OtherErr(msg) => {
                return Err(ActionError::SystemError(format!(
                    "{} at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                )));
            }
        }

        self.last_time_entered_room = Local::now();

        Ok(())
    }
    pub fn send_user_message(&mut self, message: String) -> Result<(), ActionError> {
        let time_diff = Local::now() - self.last_time_text_message_sent;
        if time_diff.num_seconds() < SPAM_PROTECTION_SEC as i64 {
            return Err(ActionError::SendMessagesTooQuick);
        }

        match self
            .user_tcp_service
            .lock()
            .unwrap()
            .send_user_text_message(message)
        {
            HandleMessageResult::Ok => {}
            HandleMessageResult::IOError(err) => match err {
                IoResult::Err(msg) => {
                    return Err(ActionError::SystemError(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                _ => {}
            },
            HandleMessageResult::OtherErr(msg) => {
                return Err(ActionError::SystemError(format!(
                    "{} at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                )));
            }
        }

        self.last_time_text_message_sent = Local::now();

        Ok(())
    }

    fn tcp_service(
        config: ClientConfig,
        username: String,
        user_tcp_service: Arc<Mutex<UserTcpService>>,
        user_udp_service: Arc<Mutex<UserUdpService>>,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
        event_sink: ExtEventSink,
        audio_service: Arc<Mutex<AudioService>>,
    ) {
        let tcp_socket =
            TcpStream::connect(format!("{}:{}", config.server_name, config.server_port));

        if tcp_socket.is_err() {
            connect_layout_sender
                .send(ConnectResult::ErrServerOffline)
                .unwrap();
            return;
        }

        let tcp_socket = tcp_socket.unwrap();
        if tcp_socket.set_nodelay(true).is_err() {
            connect_layout_sender
                .send(ConnectResult::Err(String::from(
                    "tcp_socket.set_nodelay() failed.",
                )))
                .unwrap();
            return;
        }
        if tcp_socket.set_nonblocking(true).is_err() {
            connect_layout_sender
                .send(ConnectResult::Err(String::from(
                    "tcp_socket.set_nonblocking() failed.",
                )))
                .unwrap();
            return;
        }

        let (sender, receiver) = mpsc::channel();

        // Move socket and user info to UserNetService.
        {
            let mut user_service_guard = user_tcp_service.lock().unwrap();
            user_service_guard.tcp_socket = Some(tcp_socket);
            user_service_guard.user_info = UserInfo::new(username.clone());
        }

        // Connect.
        {
            let mut user_service_guard = user_tcp_service.lock().unwrap();

            match user_service_guard.establish_secure_connection() {
                Ok(key) => {
                    let result = key.try_into();
                    if result.is_err() {
                        event_sink
                            .submit_command(
                                NETWORK_SERVICE_SYSTEM_IO_ERROR,
                                format!(
                                    "failed to convert Vec<u8> to generic array at [{}, {}]",
                                    file!(),
                                    line!()
                                ),
                                Target::Auto,
                            )
                            .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                        return;
                    }
                    user_service_guard.secret_key = result.unwrap();
                }
                Err(e) => match e {
                    HandleMessageResult::Ok => {}
                    HandleMessageResult::IOError(err) => match err {
                        IoResult::FIN => {
                            event_sink
                                .submit_command(
                                    NETWORK_SERVICE_SYSTEM_IO_ERROR,
                                    String::from("The server closed connection."),
                                    Target::Auto,
                                )
                                .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                            return;
                        }
                        IoResult::Err(msg) => {
                            event_sink
                                .submit_command(
                                    NETWORK_SERVICE_SYSTEM_IO_ERROR,
                                    format!("{} at [{}, {}]", msg, file!(), line!()),
                                    Target::Auto,
                                )
                                .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                            return;
                        }
                        _ => {}
                    },
                    HandleMessageResult::OtherErr(msg) => {
                        event_sink
                            .submit_command(
                                NETWORK_SERVICE_SYSTEM_IO_ERROR,
                                format!("{} at [{}, {}]", msg, file!(), line!()),
                                Target::Auto,
                            )
                            .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                        return;
                    }
                },
            }

            match user_service_guard.connect_user(sender) {
                ConnectResult::Ok => {
                    // Get info about all other users.
                    let mut connected_users = 0usize;
                    loop {
                        let received = receiver.recv().unwrap();
                        match received {
                            ConnectInfo::UserInfo(user_info, room, ping_ms) => {
                                connect_layout_sender
                                    .send(ConnectResult::InfoAboutOtherUser(
                                        user_info, room, ping_ms,
                                    ))
                                    .unwrap();
                                connected_users += 1;
                            }
                            ConnectInfo::RoomInfo(room_name) => {
                                connect_layout_sender
                                    .send(ConnectResult::InfoAboutRoom(room_name))
                                    .unwrap();
                            }
                            ConnectInfo::End => {
                                break;
                            }
                        }
                    }
                    connect_layout_sender.send(ConnectResult::Ok).unwrap();

                    // Include myself.
                    connected_users += 1;
                    event_sink
                        .submit_command(
                            NETWORK_SERVICE_UPDATE_CONNECTED_USERS_COUNT,
                            connected_users,
                            Target::Auto,
                        )
                        .expect(
                            "failed to submit NETWORK_SERVICE_UPDATE_CONNECTED_USERS_COUNT command",
                        );
                }
                ConnectResult::IoErr(io_error) => {
                    let mut err = io_error;
                    if let IoResult::Err(msg) = err {
                        err = IoResult::Err(format!("{} at [{}, {}]", msg, file!(), line!()));
                    }
                    connect_layout_sender
                        .send(ConnectResult::IoErr(err))
                        .unwrap();
                }
                res => {
                    connect_layout_sender.send(res).unwrap();
                    return;
                }
            }
        }

        // Start UDP service
        {
            let username_copy = username.clone();
            let server_name_copy = config.server_name.clone();
            let server_port_copy = config.server_port.clone();
            let push_to_talk_button = config.push_to_talk_key;
            let secret_key_copy = user_tcp_service.lock().unwrap().secret_key.clone();
            let event_sink_copy = event_sink.clone();
            thread::spawn(move || {
                NetService::udp_service(
                    username_copy,
                    server_name_copy,
                    server_port_copy,
                    event_sink_copy,
                    user_udp_service,
                    audio_service,
                    push_to_talk_button,
                    secret_key_copy,
                )
            });
        }

        // Read messages from server.
        loop {
            let mut _fin = false;
            let mut in_buf = vec![0u8; std::mem::size_of::<u16>()];
            loop {
                {
                    let mut user_service_guard = user_tcp_service.lock().unwrap();
                    match user_service_guard.read_from_socket(&mut in_buf) {
                        IoResult::WouldBlock => {
                            drop(user_service_guard);
                            thread::sleep(Duration::from_millis(INTERVAL_TCP_IDLE_MS));
                            continue;
                        }
                        IoResult::Ok(_) => {}
                        IoResult::FIN => {
                            _fin = true;
                            break;
                        }
                        IoResult::Err(msg) => {
                            event_sink
                                .submit_command(
                                    NETWORK_SERVICE_SYSTEM_IO_ERROR,
                                    format!("{} at [{}, {}]", msg, file!(), line!()),
                                    Target::Auto,
                                )
                                .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                            event_sink
                                .submit_command(NETWORK_SERVICE_CLEAR_ALL_USERS, (), Target::Auto)
                                .expect("failed to submit NETWORK_SERVICE_CLEAR_ALL_USERS command");
                            return;
                        }
                    }
                }

                // Got something.
                let message_size = bincode::deserialize::<u16>(&in_buf);
                if let Err(e) = message_size {
                    event_sink
                        .submit_command(
                            NETWORK_SERVICE_SYSTEM_IO_ERROR,
                            format!(
                        "bincode::deserialize failed, error: failed to decode on 'in_buf' (error: {}) at [{}, {}].\nClosing connection...",
                        e, file!(), line!()
                    ),
                            Target::Auto,
                        )
                        .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                    return;
                }
                let message_size = message_size.unwrap();

                if message_size > TCP_PACKET_MAX_SIZE {
                    event_sink
                        .submit_command(
                            NETWORK_SERVICE_SYSTEM_IO_ERROR,
                            format!(
                        "incoming packet size exceeds the maximum size ({}/{}), at [{}, {}].\nClosing connection...",
                        message_size, TCP_PACKET_MAX_SIZE, file!(), line!()
                    ),
                            Target::Auto,
                        )
                        .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                    return;
                }

                // Handle message.
                {
                    let mut user_service_guard = user_tcp_service.lock().unwrap();
                    match user_service_guard.handle_message(message_size, event_sink.clone()) {
                        HandleMessageResult::Ok => {}
                        HandleMessageResult::IOError(err) => match err {
                            IoResult::FIN => {
                                _fin = true;
                                break;
                            }
                            IoResult::Err(msg) => {
                                _fin = true;
                                event_sink
                                    .submit_command(
                                        NETWORK_SERVICE_SYSTEM_IO_ERROR,
                                        format!("{} at [{}, {}]", msg, file!(), line!()),
                                        Target::Auto,
                                    )
                                    .expect(
                                        "failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command",
                                    );
                                break;
                            }
                            _ => {}
                        },
                        HandleMessageResult::OtherErr(msg) => {
                            _fin = true;
                            event_sink
                                .submit_command(
                                    NETWORK_SERVICE_SYSTEM_IO_ERROR,
                                    format!("{} at [{}, {}]", msg, file!(), line!()),
                                    Target::Auto,
                                )
                                .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                            break;
                        }
                    }
                }
            }

            if _fin {
                break;
            }
        }

        event_sink
            .submit_command(NETWORK_SERVICE_CLEAR_ALL_USERS, (), Target::Auto)
            .expect("failed to submit NETWORK_SERVICE_CLEAR_ALL_USERS command");
    }
    fn udp_service(
        username: String,
        server_name: String,
        server_port: String,
        event_sink: ExtEventSink,
        user_udp_service: Arc<Mutex<UserUdpService>>,
        audio_service: Arc<Mutex<AudioService>>,
        push_to_talk_key: KeyCode,
        secret_key: [u8; SECRET_KEY_SIZE],
    ) {
        let udp_socket = UdpSocket::bind("0.0.0.0:0");
        if let Err(e) = udp_socket {
            event_sink
                .submit_command(
                    NETWORK_SERVICE_SYSTEM_IO_ERROR,
                    format!(
                        "UdpSocket::bind() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ),
                    Target::Auto,
                )
                .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
            return;
        }
        let udp_socket = udp_socket.unwrap();

        if let Err(e) = udp_socket.set_nonblocking(true) {
            event_sink
                .submit_command(
                    NETWORK_SERVICE_SYSTEM_IO_ERROR,
                    format!(
                        "udp_socket.set_nonblocking() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ),
                    Target::Auto,
                )
                .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
            return;
        }

        if let Err(e) = udp_socket.connect(format!("{}:{}", server_name, server_port)) {
            event_sink
                .submit_command(
                    NETWORK_SERVICE_SYSTEM_IO_ERROR,
                    format!(
                        "udp_socket.connect() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ),
                    Target::Auto,
                )
                .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
            return;
        }

        // clone socket
        {
            let res = udp_socket.try_clone();
            if let Err(e) = res {
                event_sink
                    .submit_command(
                        NETWORK_SERVICE_SYSTEM_IO_ERROR,
                        format!(
                            "udp_socket.try_clone() failed, error: {}, at [{}, {}]",
                            e,
                            file!(),
                            line!()
                        ),
                        Target::Auto,
                    )
                    .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                return;
            }

            let mut udp_service_guard = user_udp_service.lock().unwrap();
            udp_service_guard.assign_socket_and_name(res.unwrap(), username);
            udp_service_guard.secret_key = secret_key;
        }

        match user_udp_service.lock().unwrap().connect(&udp_socket) {
            Ok(()) => {}
            Err(msg) => {
                event_sink
                    .submit_command(
                        NETWORK_SERVICE_SYSTEM_IO_ERROR,
                        format!("{}, at [{}, {}]", msg, file!(), line!()),
                        Target::Auto,
                    )
                    .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                return;
            }
        }

        // Ready.
        {
            let audio_service_guard = audio_service.lock().unwrap();
            audio_service_guard.start_waiting_for_voice(
                push_to_talk_key,
                Arc::clone(audio_service_guard.net_service.as_ref().unwrap()),
                audio_service_guard.microphone_volume,
            );
        }

        loop {
            let mut packet_size_buf = vec![0u8; std::mem::size_of::<u16>()];
            let mut _peek_len = 0usize;

            loop {
                let mut _res = Result::Ok(0);
                {
                    _res = user_udp_service
                        .lock()
                        .unwrap()
                        .peek(&udp_socket, &mut packet_size_buf);
                }
                match _res {
                    Ok(_bytes) => {
                        break;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_UDP_MESSAGE_MS));
                        continue;
                    }
                    Err(e) => {
                        event_sink
                            .submit_command(
                                NETWORK_SERVICE_SYSTEM_IO_ERROR,
                                format!(
                                    "udp_socket.peek() failed, error: {}, at [{}, {}]",
                                    e,
                                    file!(),
                                    line!()
                                ),
                                Target::Auto,
                            )
                            .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                        return;
                    }
                }
            }

            // Handle message.
            // this might sleep a little (inside of handle_message())
            match user_udp_service.lock().unwrap().handle_message(
                &udp_socket,
                event_sink.clone(),
                audio_service.clone(),
            ) {
                Ok(()) => {}
                Err(msg) => {
                    event_sink
                        .submit_command(
                            NETWORK_SERVICE_SYSTEM_IO_ERROR,
                            format!("{}, at [{}, {}]", msg, file!(), line!()),
                            Target::Auto,
                        )
                        .expect("failed to submit NETWORK_SERVICE_SYSTEM_IO_ERROR command");
                    return;
                }
            }
        }
    }
}
