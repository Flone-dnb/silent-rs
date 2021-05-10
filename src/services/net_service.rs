// External.
use bytevec::{ByteDecodable, ByteEncodable};
use num_traits::FromPrimitive;

// Std.
use std::net::*;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use crate::global_params::*;
use crate::services::user_net_service::*;
use crate::InternalMessage;

pub struct ClientConfig {
    pub username: String,
    pub server_name: String,
    pub server_port: String,
    pub server_password: String,
}

#[derive(Debug)]
pub struct NetService {}

impl NetService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(
        &mut self,
        config: ClientConfig,
        username: String,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
        internal_messages: Arc<Mutex<Vec<InternalMessage>>>,
    ) {
        thread::spawn(move || {
            NetService::service(config, username, connect_layout_sender, internal_messages)
        });
    }

    fn service(
        config: ClientConfig,
        username: String,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
        internal_messages: Arc<Mutex<Vec<InternalMessage>>>,
    ) {
        let tcp_socket =
            TcpStream::connect(format!("{}:{}", config.server_name, config.server_port));

        if tcp_socket.is_err() {
            connect_layout_sender.send(ConnectResult::OtherErr(
                String::from("Can't connect to the server. Make sure the specified server and port are correct, otherwise the server might be offline.")
            )).unwrap();
            return;
        }

        let mut tcp_socket = tcp_socket.unwrap();
        if tcp_socket.set_nodelay(true).is_err() {
            connect_layout_sender
                .send(ConnectResult::OtherErr(String::from(
                    "tcp_socket.set_nodelay() failed.",
                )))
                .unwrap();
            return;
        }
        if tcp_socket.set_nonblocking(true).is_err() {
            connect_layout_sender
                .send(ConnectResult::OtherErr(String::from(
                    "tcp_socket.set_nonblocking() failed.",
                )))
                .unwrap();
            return;
        }

        let mut user_net_service = UserNetService::new();

        let (sender, receiver) = mpsc::channel();

        // Connect.
        match user_net_service.connect_user(&mut tcp_socket, username, sender) {
            ConnectResult::Ok => {
                // Get info about all other users.
                let mut connected_users = 0usize;
                loop {
                    let received = receiver.recv().unwrap();
                    if received.is_none() {
                        // End.
                        break;
                    } else {
                        connect_layout_sender
                            .send(ConnectResult::InfoAboutOtherUser(received.unwrap()))
                            .unwrap();
                        connected_users += 1;
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
            ConnectResult::Err(io_error) => {
                let mut err = io_error;
                if let IoResult::Err(msg) = err {
                    err = IoResult::Err(format!("{} at [{}, {}]", msg, file!(), line!()));
                }
                connect_layout_sender.send(ConnectResult::Err(err)).unwrap();
            }
            res => {
                connect_layout_sender.send(res).unwrap();
                return;
            }
        }

        // Read messages from server.
        loop {
            let mut fin = false;
            let mut in_buf = vec![0u8; std::mem::size_of::<u16>()];
            loop {
                match user_net_service.read_from_socket_tcp(&mut tcp_socket, &mut in_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => {}
                    IoResult::FIN => {
                        fin = true;
                        break;
                    }
                    IoResult::Err(msg) => {
                        internal_messages
                            .lock()
                            .unwrap()
                            .push(InternalMessage::SystemIOError(format!(
                                "{} at [{}, {}]",
                                msg,
                                file!(),
                                line!()
                            )));
                        return;
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
                let message = message.unwrap();
                let mut _message_id: ServerMessage = ServerMessage::NewUser;
                match FromPrimitive::from_u16(message as u16) {
                    Some(ServerMessage::NewUser) => {
                        _message_id = ServerMessage::NewUser;
                    }
                    None => {
                        fin = true;
                        internal_messages
                        .lock()
                        .unwrap()
                        .push(InternalMessage::SystemIOError(format!(
                        "FromPrimitive::from_u16() failed on 'in_buf' (value: {}) at [{}, {}].\nClosing connection...",
                        message, file!(), line!()
                    )));
                        break;
                    }
                }

                // Handle message.
                match user_net_service.handle_message(
                    _message_id,
                    &mut tcp_socket,
                    &internal_messages,
                ) {
                    HandleMessageResult::Ok => {}
                    HandleMessageResult::IOError(err) => match err {
                        IoResult::FIN => {
                            fin = true;
                            break;
                        }
                        IoResult::Err(msg) => {
                            fin = true;
                            internal_messages
                                .lock()
                                .unwrap()
                                .push(InternalMessage::SystemIOError(format!(
                                    "{} at [{}, {}",
                                    msg,
                                    file!(),
                                    line!()
                                )));
                            break;
                        }
                        _ => {}
                    },
                    HandleMessageResult::OtherErr(msg) => {
                        fin = true;
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

            if fin {
                break;
            }
        }

        internal_messages
            .lock()
            .unwrap()
            .push(InternalMessage::ClearAllUsers);
    }
}
