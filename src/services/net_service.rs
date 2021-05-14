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
}

impl NetService {
    pub fn new() -> Self {
        Self {
            user_service: Arc::new(Mutex::new(UserTcpService::new(String::from("")))),
            last_time_text_message_sent: Local::now(),
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

        self.user_service = Arc::new(Mutex::new(UserTcpService::new(server_password)));
        let user_service_copy = Arc::clone(&self.user_service);
        thread::spawn(move || {
            NetService::service(
                config,
                username,
                user_service_copy,
                connect_layout_sender,
                internal_messages,
            )
        });
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

    fn service(
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
            user_service_guard.user_info = UserInfo::new(username);
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
                        if let Some(user_info) = received {
                            connect_layout_sender
                                .send(ConnectResult::InfoAboutOtherUser(user_info))
                                .unwrap();
                            connected_users += 1;
                        } else {
                            // End.
                            break;
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
                let message_id = ServerMessage::from_u16(message);
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
}
