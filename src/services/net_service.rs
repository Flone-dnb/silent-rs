// External.
use bytevec::ByteDecodable;
use chrono::prelude::*;
use num_traits::FromPrimitive;

// Std.
use std::net::*;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use crate::global_params::*;
use crate::services::user_tcp_service::*;
use crate::services::user_udp_service::*;
use crate::InternalMessage;

pub struct ActionError {
    pub message: String,
    pub show_modal: bool,
}

pub struct ClientConfig {
    pub username: String,
    pub server_name: String,
    pub server_port: String,
    pub server_password: String,
}

#[derive(Debug)]
pub struct PasswordRetrySleep {
    pub sleep_time_start: DateTime<Local>,
    pub sleep_time_sec: usize,
    pub sleep: bool,
}

#[derive(Debug)]
pub struct NetService {
    pub user_service: Arc<Mutex<UserTcpService>>,
    pub password_retry: PasswordRetrySleep,
    last_time_text_message_sent: DateTime<Local>,
    last_time_entered_room: DateTime<Local>,
}

impl NetService {
    pub fn new() -> Self {
        Self {
            user_service: Arc::new(Mutex::new(UserTcpService::new(String::from("")))),
            last_time_text_message_sent: Local::now(),
            last_time_entered_room: Local::now(),
            password_retry: PasswordRetrySleep {
                sleep_time_start: Local::now(),
                sleep_time_sec: 0,
                sleep: false,
            },
        }
    }

    pub fn start(
        &mut self,
        config: ClientConfig,
        username: String,
        server_password: String,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
        internal_messages: Arc<Mutex<Vec<InternalMessage>>>,
    ) {
        if self.password_retry.sleep {
            let time_diff = Local::now() - self.password_retry.sleep_time_start;
            if time_diff.num_seconds() < self.password_retry.sleep_time_sec as i64 {
                return;
            }
        }

        // Start TCP service.
        self.user_service = Arc::new(Mutex::new(UserTcpService::new(server_password)));
        let user_service_copy = Arc::clone(&self.user_service);
        thread::spawn(move || {
            NetService::tcp_service(
                config,
                username,
                user_service_copy,
                connect_layout_sender,
                internal_messages,
            )
        });
    }
    pub fn enter_room(&mut self, room: &str) -> Result<(), ActionError> {
        let time_diff = Local::now() - self.last_time_entered_room;
        if time_diff.num_seconds() < SPAM_PROTECTION_SEC as i64 {
            return Err(ActionError {
                message: String::from("You can't change rooms that quickly!"),
                show_modal: true,
            });
        }

        match self.user_service.lock().unwrap().enter_room(room) {
            HandleMessageResult::Ok => {}
            HandleMessageResult::IOError(err) => match err {
                IoResult::Err(msg) => {
                    return Err(ActionError {
                        message: format!("{} at [{}, {}]", msg, file!(), line!()),
                        show_modal: false,
                    });
                }
                _ => {}
            },
            HandleMessageResult::OtherErr(msg) => {
                return Err(ActionError {
                    message: format!("{} at [{}, {}]", msg, file!(), line!()),
                    show_modal: false,
                });
            }
        }

        self.last_time_entered_room = Local::now();

        Ok(())
    }
    pub fn send_user_message(&mut self, message: String) -> Result<(), ActionError> {
        let time_diff = Local::now() - self.last_time_text_message_sent;
        if time_diff.num_seconds() < SPAM_PROTECTION_SEC as i64 {
            return Err(ActionError {
                message: String::from("You can't send messages that quick!"),
                show_modal: true,
            });
        }

        match self
            .user_service
            .lock()
            .unwrap()
            .send_user_text_message(message)
        {
            HandleMessageResult::Ok => {}
            HandleMessageResult::IOError(err) => match err {
                IoResult::Err(msg) => {
                    return Err(ActionError {
                        message: format!("{} at [{}, {}]", msg, file!(), line!()),
                        show_modal: false,
                    });
                }
                _ => {}
            },
            HandleMessageResult::OtherErr(msg) => {
                return Err(ActionError {
                    message: format!("{} at [{}, {}]", msg, file!(), line!()),
                    show_modal: false,
                });
            }
        }

        self.last_time_text_message_sent = Local::now();

        Ok(())
    }

    fn tcp_service(
        config: ClientConfig,
        username: String,
        user_service: Arc<Mutex<UserTcpService>>,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
        internal_messages: Arc<Mutex<Vec<InternalMessage>>>,
    ) {
        let tcp_socket =
            TcpStream::connect(format!("{}:{}", config.server_name, config.server_port));

        if tcp_socket.is_err() {
            connect_layout_sender.send(ConnectResult::Err(
                String::from("Can't connect to the server. Make sure the specified server and port are correct, otherwise the server might be offline.")
            )).unwrap();
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

        // Move socket and userinfo to UserNetService.
        {
            let mut user_service_guard = user_service.lock().unwrap();
            user_service_guard.tcp_socket = Some(tcp_socket);
            user_service_guard.user_info = UserInfo::new(username.clone());
        }

        // Connect.
        {
            let mut user_service_guard = user_service.lock().unwrap();
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
                    internal_messages
                        .lock()
                        .unwrap()
                        .push(InternalMessage::RefreshConnectedUsersCount(connected_users));
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
            let internam_messages_copy = Arc::clone(&internal_messages);
            thread::spawn(move || {
                NetService::udp_service(
                    username_copy,
                    server_name_copy,
                    server_port_copy,
                    internam_messages_copy,
                )
            });
        }

        // Read messages from server.
        loop {
            let mut _fin = false;
            let mut in_buf = vec![0u8; std::mem::size_of::<u16>()];
            loop {
                {
                    let mut user_service_guard = user_service.lock().unwrap();
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
                            let mut internal_messages_guard = internal_messages.lock().unwrap();
                            internal_messages_guard.push(InternalMessage::SystemIOError(format!(
                                "{} at [{}, {}]",
                                msg,
                                file!(),
                                line!()
                            )));
                            internal_messages_guard.push(InternalMessage::ClearAllUsers);
                            return;
                        }
                    }
                }

                // Got something.
                let message = u16::decode::<u16>(&in_buf);
                if let Err(e) = message {
                    internal_messages
                        .lock()
                        .unwrap()
                        .push(InternalMessage::SystemIOError(format!(
                        "u16::decode::<u16>() failed, error: failed to decode on 'in_buf' (error: {}) at [{}, {}].\nClosing connection...",
                        e, file!(), line!()
                    )));
                    return;
                }
                let message = message.unwrap() as u16;
                let message_id = ServerMessageTcp::from_u16(message);
                if message_id.is_none() {
                    _fin = true;
                    internal_messages
                        .lock()
                        .unwrap()
                        .push(InternalMessage::SystemIOError(format!(
                        "FromPrimitive::from_u16() failed on 'in_buf' (value: {}) at [{}, {}].\nClosing connection...",
                        message, file!(), line!()
                    )));
                    break;
                }
                let message_id = message_id.unwrap();

                // Handle message.
                {
                    let mut user_service_guard = user_service.lock().unwrap();
                    match user_service_guard.handle_message(message_id, &internal_messages) {
                        HandleMessageResult::Ok => {}
                        HandleMessageResult::IOError(err) => match err {
                            IoResult::FIN => {
                                _fin = true;
                                break;
                            }
                            IoResult::Err(msg) => {
                                _fin = true;
                                internal_messages.lock().unwrap().push(
                                    InternalMessage::SystemIOError(format!(
                                        "{} at [{}, {}",
                                        msg,
                                        file!(),
                                        line!()
                                    )),
                                );
                                break;
                            }
                            _ => {}
                        },
                        HandleMessageResult::OtherErr(msg) => {
                            _fin = true;
                            internal_messages
                                .lock()
                                .unwrap()
                                .push(InternalMessage::SystemIOError(format!(
                                    "{} at [{}, {}]",
                                    msg,
                                    file!(),
                                    line!()
                                )));
                            break;
                        }
                    }
                }
            }

            if _fin {
                break;
            }
        }

        internal_messages
            .lock()
            .unwrap()
            .push(InternalMessage::ClearAllUsers);
    }
    fn udp_service(
        username: String,
        server_name: String,
        server_port: String,
        internal_messages: Arc<Mutex<Vec<InternalMessage>>>,
    ) {
        let udp_socket = UdpSocket::bind("127.0.0.1:0"); // random port
        if let Err(e) = udp_socket {
            internal_messages
                .lock()
                .unwrap()
                .push(InternalMessage::SystemIOError(format!(
                    "UdpSocket::bind() failed, error: {}, at [{}, {}]",
                    e,
                    file!(),
                    line!()
                )));
            return;
        }
        let udp_socket = udp_socket.unwrap();

        if let Err(e) = udp_socket.set_nonblocking(true) {
            internal_messages
                .lock()
                .unwrap()
                .push(InternalMessage::SystemIOError(format!(
                    "udp_socket.set_nonblocking() failed, error: {}, at [{}, {}]",
                    e,
                    file!(),
                    line!()
                )));
            return;
        }

        if let Err(e) = udp_socket.connect(format!("{}:{}", server_name, server_port)) {
            internal_messages
                .lock()
                .unwrap()
                .push(InternalMessage::SystemIOError(format!(
                    "udp_socket.connect() failed, error: {}, at [{}, {}]",
                    e,
                    file!(),
                    line!()
                )));
            return;
        }

        let mut user_udp_service = UserUdpService::new();
        match user_udp_service.connect(&udp_socket, &username) {
            Ok(()) => {}
            Err(msg) => {
                internal_messages
                    .lock()
                    .unwrap()
                    .push(InternalMessage::SystemIOError(format!(
                        "{}, at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                return;
            }
        }

        // Ready.

        loop {
            let mut in_buf = vec![0u8; IN_UDP_BUFFER_SIZE];
            let mut _peek_len = 0usize;

            loop {
                match user_udp_service.peek(&udp_socket, &mut in_buf) {
                    Ok(n) => {
                        if n > 0 {
                            _peek_len = n;
                            break;
                        }
                    }
                    Err(msg) => {
                        internal_messages
                            .lock()
                            .unwrap()
                            .push(InternalMessage::SystemIOError(format!(
                                "{}, at [{}, {}]",
                                msg,
                                file!(),
                                line!()
                            )));
                        return;
                    }
                }
            }

            match user_udp_service.recv(&udp_socket, &mut in_buf, _peek_len) {
                Ok(()) => {}
                Err(msg) => {
                    internal_messages
                        .lock()
                        .unwrap()
                        .push(InternalMessage::SystemIOError(format!(
                            "{}, at [{}, {}]",
                            msg,
                            file!(),
                            line!()
                        )));
                    return;
                }
            }

            match user_udp_service.handle_message(&udp_socket, &mut in_buf, &internal_messages) {
                Ok(()) => {}
                Err(msg) => {
                    internal_messages
                        .lock()
                        .unwrap()
                        .push(InternalMessage::SystemIOError(format!(
                            "{}, at [{}, {}]",
                            msg,
                            file!(),
                            line!()
                        )));
                    return;
                }
            }
        }

        // End.
    }
}
