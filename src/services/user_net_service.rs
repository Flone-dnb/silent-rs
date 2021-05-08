// External.
use bytevec::{ByteDecodable, ByteEncodable};
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::FromPrimitive;

// Std.
use std::io::prelude::*;
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use crate::global_params::*;
use crate::InternalMessage;

pub enum UserState {
    NotConnected,
    Connected,
}

#[derive(FromPrimitive)]
enum ConnectServerAnswer {
    Ok = 0,
    WrongVersion = 1,
    UsernameTaken = 2,
}

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum ServerMessage {
    NewUser = 0,
}

#[derive(Debug, PartialEq)]
pub struct UserInfo {
    pub username: String,
}

impl UserInfo {
    pub fn new(username: String) -> Self {
        UserInfo { username }
    }
}

#[derive(Debug, PartialEq)]
pub enum ConnectResult {
    Ok,
    Err(IoResult),
    OtherErr(String),
    InfoAboutOtherUser(UserInfo),
}

#[derive(Debug, PartialEq)]
pub enum IoResult {
    Ok(usize),
    WouldBlock,
    FIN,
    Err(String),
}

#[derive(Debug, PartialEq)]
pub enum HandleMessageResult {
    Ok,
    IOError(IoResult),
    OtherErr(String),
}

pub struct UserNetService {
    pub user_state: UserState,
    pub io_tcp_mutex: Mutex<()>,
}

impl UserNetService {
    pub fn new() -> Self {
        UserNetService {
            user_state: UserState::NotConnected,
            io_tcp_mutex: Mutex::new(()),
        }
    }
    pub fn read_from_socket_tcp(&self, socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        let _io_tcp_guard = self.io_tcp_mutex.lock().unwrap();

        // (non-blocking)
        match socket.read(buf) {
            Ok(0) => {
                return IoResult::FIN;
            }
            Ok(n) => {
                if n != buf.len() {
                    return IoResult::Err(format!(
                        "socket try_read() failed, error: failed to read 'buf' size (got: {}, expected: {})", n, buf.len()
                    ));
                }

                return IoResult::Ok(n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return IoResult::WouldBlock;
            }
            Err(e) => {
                return IoResult::Err(String::from(format!(
                    "socket try_read() failed, error: {}",
                    e
                )));
            }
        };
    }
    pub fn write_to_socket_tcp(&self, socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        let _io_tcp_guard = self.io_tcp_mutex.lock().unwrap();

        // (non-blocking)
        match socket.write(buf) {
            Ok(0) => {
                return IoResult::FIN;
            }
            Ok(n) => {
                if n != buf.len() {
                    return IoResult::Err(format!(
                        "socket try_write() failed, error: failed to read 'buf' size (got: {}, expected: {})", n, buf.len()
                    ));
                }

                return IoResult::Ok(n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return IoResult::WouldBlock;
            }
            Err(e) => {
                return IoResult::Err(String::from(format!(
                    "socket try_write() failed, error: {}",
                    e
                )));
            }
        };
    }
    pub fn handle_message(
        &mut self,
        message: ServerMessage,
        socket: &mut TcpStream,
        internal_messages_ok_only: &Arc<Mutex<Vec<InternalMessage>>>,
    ) -> HandleMessageResult {
        match message {
            ServerMessage::NewUser => {
                // Get username len.
                let mut username_len_buf = vec![0u8; std::mem::size_of::<u16>()];
                loop {
                    match self.read_from_socket_tcp(socket, &mut username_len_buf) {
                        IoResult::WouldBlock => {
                            thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                            continue;
                        }
                        IoResult::Ok(_) => break,
                        res => return HandleMessageResult::IOError(res),
                    };
                }

                let username_len = u16::decode::<u16>(&username_len_buf);
                if username_len.is_err() {
                    return HandleMessageResult::OtherErr(format!(
                        "decode::<u16>() failed on 'username_len_buf'."
                    ));
                }
                let username_len = username_len.unwrap();

                let mut username = vec![0u8; username_len as usize];
                loop {
                    match self.read_from_socket_tcp(socket, &mut username) {
                        IoResult::WouldBlock => {
                            thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                            continue;
                        }
                        IoResult::Ok(_) => break,
                        res => return HandleMessageResult::IOError(res),
                    };
                }

                let new_username_str = String::from_utf8(username);
                if new_username_str.is_err() {
                    return HandleMessageResult::OtherErr(format!(
                        "String::from_utf8() failed on 'username'."
                    ));
                }

                internal_messages_ok_only
                    .lock()
                    .unwrap()
                    .push(InternalMessage::NewUser(new_username_str.unwrap()));
            }
        }

        HandleMessageResult::Ok
    }
    pub fn connect_user(
        &mut self,
        socket: &mut TcpStream,
        username: String,
        info_sender: std::sync::mpsc::Sender<Option<UserInfo>>,
    ) -> ConnectResult {
        // Prepare initial send buffer:
        // (u16): size of the version string,
        // (size): version string,
        // (u16): size of the username,
        // (size): username string,
        let ver_str_len = env!("CARGO_PKG_VERSION").len() as u16;
        let name_str_len = username.len() as u16;

        // Convert to buffers.
        let mut ver_str_len_buf = u16::encode::<u16>(&ver_str_len).unwrap();
        let mut ver_str_buf = Vec::from(env!("CARGO_PKG_VERSION").as_bytes());
        let mut name_str_len_buf = u16::encode::<u16>(&name_str_len).unwrap();
        let mut name_str_buf = Vec::from(username.as_bytes());

        // Move all buffers to one big buffer.
        let mut out_buffer: Vec<u8> = Vec::new();
        out_buffer.append(&mut ver_str_len_buf);
        out_buffer.append(&mut ver_str_buf);
        out_buffer.append(&mut name_str_len_buf);
        out_buffer.append(&mut name_str_buf);

        // Send this buffer.
        loop {
            match self.write_to_socket_tcp(socket, &mut out_buffer) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_CONNECT_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return ConnectResult::Err(res),
            };
        }

        // Wait for answer.
        let mut in_buf = vec![0u8; std::mem::size_of::<u16>()];
        loop {
            match self.read_from_socket_tcp(socket, &mut in_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_CONNECT_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return ConnectResult::Err(res),
            }
        }

        // See answer.
        let answer_id = u16::decode::<u16>(&in_buf).unwrap();
        match FromPrimitive::from_i32(answer_id as i32) {
            Some(ConnectServerAnswer::Ok) => {}
            Some(ConnectServerAnswer::WrongVersion) => {
                // Get correct version string (get size first).
                loop {
                    match self.read_from_socket_tcp(socket, &mut in_buf) {
                        IoResult::WouldBlock => {
                            thread::sleep(Duration::from_millis(INTERVAL_TCP_CONNECT_MS));
                            continue;
                        }
                        IoResult::Ok(_bytes) => break,
                        res => return ConnectResult::Err(res),
                    }
                }
                let required_ver_str_size = u16::decode::<u16>(&in_buf).unwrap();

                // Get correct version string.
                let mut required_ver_str_buf = vec![0u8; required_ver_str_size as usize];
                loop {
                    match self.read_from_socket_tcp(socket, &mut required_ver_str_buf) {
                        IoResult::WouldBlock => {
                            thread::sleep(Duration::from_millis(INTERVAL_TCP_CONNECT_MS));
                            continue;
                        }
                        IoResult::Ok(_bytes) => break,
                        res => return ConnectResult::Err(res),
                    }
                }
                let ver_str = std::str::from_utf8(&required_ver_str_buf);
                if ver_str.is_err(){
                    return ConnectResult::OtherErr(String::from("from_utf8() failed on required_ver_str_buf"));
                }
                return ConnectResult::OtherErr(
                    String::from(
                        format!(
                            "Your client version ({}) is not supported by this server. The server supports version ({}).",
                            env!("CARGO_PKG_VERSION"),
                            std::str::from_utf8(&required_ver_str_buf).unwrap()
                        )
                    ));
            }
            Some(ConnectServerAnswer::UsernameTaken) =>
            return ConnectResult::OtherErr(String::from("Somebody with your username already persists on the server. Please, choose another username.")),
            None => {
                return ConnectResult::OtherErr(String::from("FromPrimitive::from_i32 failed()"))
            }
        }

        // Ok.
        // Read info about other users.
        // Read user count.
        let mut users_count_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket_tcp(socket, &mut users_count_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_CONNECT_MS));
                    continue;
                }
                IoResult::Ok(_) => break,
                res => return ConnectResult::Err(res),
            }
        }

        let user_count = u64::decode::<u64>(&users_count_buf);
        if user_count.is_err() {
            return ConnectResult::OtherErr(String::from(
                "decode::<u64> failed on users_count_buf",
            ));
        }
        let user_count = user_count.unwrap();

        for _ in 0..user_count {
            // Read username len.
            let mut username_len_buf = vec![0u8; std::mem::size_of::<u16>()];
            loop {
                match self.read_from_socket_tcp(socket, &mut username_len_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_CONNECT_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::Err(res),
                }
            }
            let username_len = u16::decode::<u16>(&username_len_buf);
            if username_len.is_err() {
                return ConnectResult::OtherErr(String::from(
                    "decode::<u16> failed on username_len_buf",
                ));
            }
            let username_len = username_len.unwrap();

            // Read username.
            let mut username_buf = vec![0u8; username_len as usize];
            loop {
                match self.read_from_socket_tcp(socket, &mut username_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_CONNECT_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::Err(res),
                }
            }
            let username = std::str::from_utf8(&username_buf);
            if username.is_err() {
                return ConnectResult::OtherErr(String::from("from_utf8() failed on username_buf"));
            }

            info_sender
                .send(Some(UserInfo::new(String::from(username.unwrap()))))
                .unwrap();
        }

        info_sender.send(None).unwrap(); // End.

        self.user_state = UserState::Connected;

        return ConnectResult::Ok;
    }
}
