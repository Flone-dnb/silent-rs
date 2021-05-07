use std::net::*;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

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
pub struct NetService {
    stop_pending: Arc<Mutex<bool>>,
    stop_receiver: Option<std::sync::mpsc::Receiver<()>>,
}

impl NetService {
    pub fn new() -> Self {
        Self {
            stop_pending: Arc::new(Mutex::new(false)),
            stop_receiver: None,
        }
    }

    pub fn start(
        &mut self,
        config: ClientConfig,
        username: String,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
        internal_messages: Arc<Mutex<Vec<InternalMessage>>>,
    ) {
        let (sender, receiver) = mpsc::channel();

        let stop_pending_copy = Arc::clone(&self.stop_pending);
        thread::spawn(move || {
            NetService::service(
                config,
                username,
                connect_layout_sender,
                internal_messages,
                stop_pending_copy,
                sender,
            )
        });
        self.stop_receiver = Some(receiver);
    }

    pub fn stop(&mut self) {
        match &self.stop_receiver {
            Some(r) => {
                *self.stop_pending.lock().unwrap() = true;
                r.recv().unwrap();
            }
            None => {}
        }
    }

    fn service(
        config: ClientConfig,
        username: String,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
        internal_messages: Arc<Mutex<Vec<InternalMessage>>>,
        stop_pending: Arc<Mutex<bool>>,
        stop_sender: std::sync::mpsc::Sender<()>,
    ) {
        let stream = TcpStream::connect(format!("{}:{}", config.server_name, config.server_port));

        if stream.is_err() {
            connect_layout_sender.send(ConnectResult::OtherErr(
                String::from("Can't connect to the server. Make sure the specified server and port are correct, otherwise the server might be offline.")
            )).unwrap();
            return;
        }

        let mut stream = stream.unwrap();
        if stream.set_nodelay(true).is_err() {
            connect_layout_sender
                .send(ConnectResult::OtherErr(String::from(
                    "stream.set_nodelay() failed.",
                )))
                .unwrap();
            return;
        }
        if stream.set_nonblocking(true).is_err() {
            connect_layout_sender
                .send(ConnectResult::OtherErr(String::from(
                    "stream.set_nonblocking() failed.",
                )))
                .unwrap();
            return;
        }

        let mut user_net_service = UserNetService::new();

        let (sender, receiver) = mpsc::channel();

        match user_net_service.connect_user(&mut stream, username, sender) {
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
            res => {
                connect_layout_sender.send(res).unwrap();
                return;
            }
        }

        loop {
            let mut fin = false;
            let mut in_buf = vec![0u8; std::mem::size_of::<u16>()];
            loop {
                match user_net_service.read_from_socket(&mut stream, &mut in_buf) {
                    IoResult::WouldBlock => {
                        {
                            let guard = stop_pending.lock().unwrap();
                            if *guard == true {
                                stop_sender.send(()).unwrap();
                                return;
                            }
                        }
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    IoResult::FIN => fin = true,
                    IoResult::Err(msg) => {
                        internal_messages
                        .lock()
                        .unwrap()
                        .push(
                            InternalMessage::SystemIOError(format!("An error occurred, user_net_service.read_from_socket() failed with error: {}", msg))
                        );
                        return;
                    }
                }
            }

            if fin {
                break;
            }
        }
    }
}
