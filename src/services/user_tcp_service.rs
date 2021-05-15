// External.
use bytevec::{ByteDecodable, ByteEncodable};
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::FromPrimitive;
use num_traits::ToPrimitive;

// Std.
use std::io::prelude::*;
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use crate::global_params::*;
use crate::InternalMessage;

#[derive(Debug)]
pub enum UserState {
    NotConnected,
    Connected,
}

#[derive(FromPrimitive)]
enum ConnectServerAnswer {
    Ok = 0,
    WrongVersion = 1,
    UsernameTaken = 2,
    WrongPassword = 3,
}

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum ServerMessageTcp {
    UserConnected = 0,
    UserDisconnected = 1,
    UserMessage = 2,
    UserEntersRoom = 3,
    KeepAliveCheck = 4,
    UserPing = 5,
}

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum ClientMessageTcp {
    UserMessage = 0,
    EnterRoom = 1,
    KeepAliveCheck = 2,
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
    IoErr(IoResult),
    Err(String),
    SleepWithErr {
        message: String,
        sleep_in_sec: usize,
    },
    InfoAboutOtherUser(UserInfo, String, u16),
    InfoAboutRoom(String),
}

pub enum ConnectInfo {
    UserInfo(UserInfo, String, u16),
    RoomInfo(String),
    End,
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

#[derive(Debug)]
pub struct UserTcpService {
    pub user_state: UserState,
    pub user_info: UserInfo,
    pub server_password: String,
    pub tcp_socket: Option<TcpStream>,
    pub io_tcp_mutex: Mutex<()>,
}

impl UserTcpService {
    pub fn new(server_password: String) -> Self {
        UserTcpService {
            user_state: UserState::NotConnected,
            tcp_socket: None,
            server_password: server_password,
            user_info: UserInfo {
                username: String::from(""),
            },
            io_tcp_mutex: Mutex::new(()),
        }
    }
    pub fn enter_room(&mut self, room: &str) -> HandleMessageResult {
        if self.tcp_socket.is_none() {
            return HandleMessageResult::OtherErr(format!(
                "UserNetService::send_user_text_message() failed, error: tcp_socket was None at [{}, {}]", file!(), line!()
            ));
        }

        // Send data:
        // (u16) - data ID (enter room)
        // (u16) - username.len()
        // (size) - username
        // (u8) - room.len()
        // (size) - room name

        // Prepare data ID buffer.
        let data_id = ClientMessageTcp::EnterRoom.to_u16();
        if data_id.is_none() {
            return HandleMessageResult::OtherErr(format!(
                "ClientMessage::EnterRoom.to_u16() failed at [{}, {}]",
                file!(),
                line!()
            ));
        }
        let data_id = data_id.unwrap();
        let data_id_buf = u16::encode::<u16>(&data_id);
        if let Err(e) = data_id_buf {
            return HandleMessageResult::OtherErr(format!(
                "u16::encode::<u16>() failed on value {}, error: {} at [{}, {}]",
                data_id,
                e,
                file!(),
                line!()
            ));
        }
        let mut data_id_buf = data_id_buf.unwrap();

        // Prepare username len buffer.
        let username_len = self.user_info.username.len() as u16;
        let username_len_buf = u16::encode::<u16>(&username_len);
        if let Err(e) = username_len_buf {
            return HandleMessageResult::OtherErr(format!(
                "u16::encode::<u16>() failed on value {}, error: {} at [{}, {}]",
                username_len,
                e,
                file!(),
                line!()
            ));
        }
        let mut username_len_buf = username_len_buf.unwrap();

        // Prepare username buffer.
        let mut username_buf = Vec::from(self.user_info.username.as_bytes());

        // Prepare room name len buffer.
        let room_name_len = room.len() as u8;
        let room_len_buf = u8::encode::<u8>(&room_name_len);
        if let Err(e) = room_len_buf {
            return HandleMessageResult::OtherErr(format!(
                "u16::encode::<u8>() failed on value {}, error: {} at [{}, {}]",
                username_len,
                e,
                file!(),
                line!()
            ));
        }
        let mut room_len_buf = room_len_buf.unwrap();

        // Prepare username buffer.
        let mut room_buf = Vec::from(room.as_bytes());

        // Merge all to one buffer.
        let mut out_buffer: Vec<u8> = Vec::new();
        out_buffer.append(&mut data_id_buf);
        out_buffer.append(&mut username_len_buf);
        out_buffer.append(&mut username_buf);
        out_buffer.append(&mut room_len_buf);
        out_buffer.append(&mut room_buf);

        // Send to server.
        loop {
            match self.write_to_socket(&mut out_buffer) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return HandleMessageResult::IOError(res),
            }
        }

        // TODO: wait for result (if password and etc.)
        // return Err if not entered!!! (see main.rs at 'MainLayoutMessage::RoomItemPressed')

        HandleMessageResult::Ok
    }
    pub fn send_user_text_message(&mut self, message: String) -> HandleMessageResult {
        if self.tcp_socket.is_none() {
            return HandleMessageResult::OtherErr(format!(
                "UserNetService::send_user_text_message() failed, error: tcp_socket was None at [{}, {}]", file!(), line!()
            ));
        }

        // Send data:
        // (u16) - data ID (user message)
        // (u16) - username.len()
        // (size) - username
        // (u16) - message.len()
        // (size) - message

        // Prepare data ID buffer.
        let data_id = ClientMessageTcp::UserMessage.to_u16();
        if data_id.is_none() {
            return HandleMessageResult::OtherErr(format!(
                "ClientMessage::UserMessage.to_u16() failed at [{}, {}]",
                file!(),
                line!()
            ));
        }
        let data_id = data_id.unwrap();
        let data_id_buf = u16::encode::<u16>(&data_id);
        if let Err(e) = data_id_buf {
            return HandleMessageResult::OtherErr(format!(
                "u16::encode::<u16>() failed on value {}, error: {} at [{}, {}]",
                data_id,
                e,
                file!(),
                line!()
            ));
        }
        let mut data_id_buf = data_id_buf.unwrap();

        // Prepare username len buffer.
        let username_len = self.user_info.username.len() as u16;
        let username_len_buf = u16::encode::<u16>(&username_len);
        if let Err(e) = username_len_buf {
            return HandleMessageResult::OtherErr(format!(
                "u16::encode::<u16>() failed on value {}, error: {} at [{}, {}]",
                username_len,
                e,
                file!(),
                line!()
            ));
        }
        let mut username_len_buf = username_len_buf.unwrap();

        // Prepare username buffer.
        let mut username_buf = Vec::from(self.user_info.username.as_bytes());

        // Prepare message len buffer.
        let message_len = message.len() as u16;
        let message_len_buf = u16::encode::<u16>(&message_len);
        if let Err(e) = message_len_buf {
            return HandleMessageResult::OtherErr(format!(
                "u16::encode::<u16>() failed on value {}, error: {} at [{}, {}]",
                message_len,
                e,
                file!(),
                line!()
            ));
        }
        let mut message_len_buf = message_len_buf.unwrap();

        // Prepare message buffer.
        let mut message_buf = Vec::from(message.as_bytes());

        // Merge all to one buffer.
        let mut out_buffer: Vec<u8> = Vec::new();
        out_buffer.append(&mut data_id_buf);
        out_buffer.append(&mut username_len_buf);
        out_buffer.append(&mut username_buf);
        out_buffer.append(&mut message_len_buf);
        out_buffer.append(&mut message_buf);

        // Send to server.
        loop {
            match self.write_to_socket(&mut out_buffer) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return HandleMessageResult::IOError(res),
            }
        }

        HandleMessageResult::Ok
    }
    pub fn read_from_socket(&mut self, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(format!(
                "An error occurred at UserTcpService::read_from_socket(), error: passed 'buf' has 0 len at [{}, {}]", file!(), line!()
            ));
        }

        if self.tcp_socket.is_none() {
            return IoResult::Err(format!(
                "UserNetService::read_from_socket_tcp() failed, error: tcp_socket was None at [{}, {}]",
                file!(),
                line!()
            ));
        }

        let _io_tcp_guard = self.io_tcp_mutex.lock().unwrap();

        // (non-blocking)
        match self.tcp_socket.as_mut().unwrap().read(buf) {
            Ok(0) => {
                return IoResult::FIN;
            }
            Ok(n) => {
                if n != buf.len() {
                    return IoResult::Err(format!(
                        "TcpStream::read() failed, error: failed to read to 'buf' (got: {}, expected: {}) at [{}, {}]",
                        n, buf.len(), file!(), line!()
                    ));
                }

                return IoResult::Ok(n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return IoResult::WouldBlock;
            }
            Err(e) => {
                return IoResult::Err(format!(
                    "TcpStream::read() failed, error: {} at [{}, {}]",
                    e,
                    file!(),
                    line!()
                ));
            }
        };
    }
    pub fn write_to_socket(&mut self, buf: &mut [u8]) -> IoResult {
        if self.tcp_socket.is_none() {
            return IoResult::Err(format!(
                "UserNetService::write_to_socket_tcp() failed, error: tcp_socket was None at [{}, {}]",
                file!(),
                line!()
            ));
        }

        let _io_tcp_guard = self.io_tcp_mutex.lock().unwrap();

        // (non-blocking)
        match self.tcp_socket.as_mut().unwrap().write(buf) {
            Ok(0) => {
                return IoResult::FIN;
            }
            Ok(n) => {
                if n != buf.len() {
                    return IoResult::Err(format!(
                        "TcpStream::write() failed, error: failed to write to 'buf' (got: {}, expected: {}) at [{}, {}]",
                        n, buf.len(), file!(), line!()
                    ));
                }

                return IoResult::Ok(n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return IoResult::WouldBlock;
            }
            Err(e) => {
                return IoResult::Err(format!(
                    "TcpStream::write() failed, error: {} at [{}, {}]",
                    e,
                    file!(),
                    line!()
                ));
            }
        };
    }
    pub fn handle_message(
        &mut self,
        message: ServerMessageTcp,
        internal_messages_ok_only: &Arc<Mutex<Vec<InternalMessage>>>,
    ) -> HandleMessageResult {
        if message == ServerMessageTcp::KeepAliveCheck {
            // resend this
            if let Err(e) = self.send_keep_alive_check() {
                return HandleMessageResult::IOError(e);
            } else {
                println!("keep alive ok");
                return HandleMessageResult::Ok;
            }
        }

        let mut username = String::new();
        match self.read_u16_and_string_from_socket() {
            Ok(name) => username = name,
            Err(io_e) => match io_e {
                IoResult::FIN => return HandleMessageResult::IOError(IoResult::FIN),
                IoResult::Err(msg) => {
                    return HandleMessageResult::IOError(IoResult::Err(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )))
                }
                _ => {}
            },
        }

        match message {
            ServerMessageTcp::UserConnected => {
                internal_messages_ok_only
                    .lock()
                    .unwrap()
                    .push(InternalMessage::UserConnected(username));
            }
            ServerMessageTcp::UserDisconnected => {
                internal_messages_ok_only
                    .lock()
                    .unwrap()
                    .push(InternalMessage::UserDisconnected(username));
            }
            ServerMessageTcp::UserMessage => {
                let mut message = String::new();
                match self.read_u16_and_string_from_socket() {
                    Ok(name) => message = name,
                    Err(io_e) => match io_e {
                        IoResult::FIN => return HandleMessageResult::IOError(IoResult::FIN),
                        IoResult::Err(msg) => {
                            return HandleMessageResult::IOError(IoResult::Err(format!(
                                "{} at [{}, {}]",
                                msg,
                                file!(),
                                line!()
                            )))
                        }
                        _ => {}
                    },
                }

                internal_messages_ok_only
                    .lock()
                    .unwrap()
                    .push(InternalMessage::UserMessage { username, message });
            }
            ServerMessageTcp::UserPing => {
                let mut ping_buf = vec![0u8; std::mem::size_of::<u16>()];
                loop {
                    match self.read_from_socket(&mut ping_buf) {
                        IoResult::WouldBlock => {
                            thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                            continue;
                        }
                        IoResult::Ok(_bytes) => break,
                        res => return HandleMessageResult::IOError(res),
                    }
                }
                let ping_ms = u16::decode::<u16>(&ping_buf);
                if let Err(e) = ping_ms {
                    return HandleMessageResult::IOError(IoResult::Err(format!(
                        "u16::decode::<u16>() failed, error: {} at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    )));
                }
                let ping_ms = ping_ms.unwrap();

                internal_messages_ok_only
                    .lock()
                    .unwrap()
                    .push(InternalMessage::UserPing { username, ping_ms });
            }
            ServerMessageTcp::UserEntersRoom => {
                let mut room = String::new();
                match self.read_u8_and_string_from_socket() {
                    Ok(name) => room = name,
                    Err(io_e) => match io_e {
                        IoResult::FIN => return HandleMessageResult::IOError(IoResult::FIN),
                        IoResult::Err(msg) => {
                            return HandleMessageResult::IOError(IoResult::Err(format!(
                                "{} at [{}, {}]",
                                msg,
                                file!(),
                                line!()
                            )))
                        }
                        _ => {}
                    },
                }

                internal_messages_ok_only
                    .lock()
                    .unwrap()
                    .push(InternalMessage::MoveUserToRoom {
                        username,
                        room_to: room,
                    });
            }
            ServerMessageTcp::KeepAliveCheck => {} // already checked this message above
        }

        HandleMessageResult::Ok
    }

    pub fn connect_user(
        &mut self,
        info_sender: std::sync::mpsc::Sender<ConnectInfo>,
    ) -> ConnectResult {
        // Prepare initial send buffer:
        // (u16): size of the version string,
        // (size): version string,
        // (u16): size of the username,
        // (size): username string,
        // (u16): size of the password string,
        // (size): password string.
        let ver_str_len = env!("CARGO_PKG_VERSION").len() as u16;
        let name_str_len = self.user_info.username.len() as u16;

        // Convert to buffers.
        let mut ver_str_len_buf = u16::encode::<u16>(&ver_str_len).unwrap();
        let mut ver_str_buf = Vec::from(env!("CARGO_PKG_VERSION").as_bytes());
        let mut name_str_len_buf = u16::encode::<u16>(&name_str_len).unwrap();
        let mut name_str_buf = Vec::from(self.user_info.username.as_bytes());
        // server password len
        let server_pass_len = self.server_password.len() as u16;
        let pass_str_len_buf = u16::encode::<u16>(&server_pass_len);
        if let Err(e) = pass_str_len_buf {
            return ConnectResult::Err(format!(
                "u16::encode::<u16>() failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        let mut pass_str_len_buf = pass_str_len_buf.unwrap();

        // Move all buffers to one big buffer.
        let mut out_buffer: Vec<u8> = Vec::new();
        out_buffer.append(&mut ver_str_len_buf);
        out_buffer.append(&mut ver_str_buf);
        out_buffer.append(&mut name_str_len_buf);
        out_buffer.append(&mut name_str_buf);
        out_buffer.append(&mut pass_str_len_buf);
        if !self.server_password.is_empty() {
            // append password
            let mut password_buf = Vec::from(self.server_password.as_bytes());
            out_buffer.append(&mut password_buf);
        }

        // Send this buffer.
        loop {
            match self.write_to_socket(&mut out_buffer) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return ConnectResult::IoErr(res),
            };
        }

        // Wait for answer.
        let mut in_buf = vec![0u8; std::mem::size_of::<u16>()];
        loop {
            match self.read_from_socket(&mut in_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return ConnectResult::IoErr(res),
            }
        }

        // See answer.
        let answer_id = u16::decode::<u16>(&in_buf).unwrap();
        match FromPrimitive::from_i32(answer_id as i32) {
            Some(ConnectServerAnswer::Ok) => {}
            Some(ConnectServerAnswer::WrongPassword) =>{
                return ConnectResult::SleepWithErr{
                    message: format!("Server reply: wrong password, try again after {} seconds...", PASSWORD_RETRY_DELAY_SEC),
                    sleep_in_sec: PASSWORD_RETRY_DELAY_SEC
                };
            }
            Some(ConnectServerAnswer::WrongVersion) => {
                // Get correct version string (get size first).
                loop {
                    match self.read_from_socket(&mut in_buf) {
                        IoResult::WouldBlock => {
                            thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                            continue;
                        }
                        IoResult::Ok(_bytes) => break,
                        res => return ConnectResult::IoErr(res),
                    }
                }
                let required_ver_str_size = u16::decode::<u16>(&in_buf).unwrap();

                // Get correct version string.
                let mut required_ver_str_buf = vec![0u8; required_ver_str_size as usize];
                loop {
                    match self.read_from_socket(&mut required_ver_str_buf) {
                        IoResult::WouldBlock => {
                            thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                            continue;
                        }
                        IoResult::Ok(_bytes) => break,
                        res => return ConnectResult::IoErr(res),
                    }
                }
                let ver_str = std::str::from_utf8(&required_ver_str_buf);
                if let Err(e) = ver_str{
                    return ConnectResult::Err(
                        format!("std::str::from_utf8() failed, error: failed to convert on 'required_ver_str_buf' (error: {}) at [{}, {}]",
                        e, file!(), line!()));
                }
                return ConnectResult::Err(
                        format!(
                            "Server reply: your client version ({}) is not supported by this server, the server supports version ({}).",
                            env!("CARGO_PKG_VERSION"),
                            std::str::from_utf8(&required_ver_str_buf).unwrap()
                        )
                    );
            }
            Some(ConnectServerAnswer::UsernameTaken) =>
            return ConnectResult::Err(String::from("Server reply: somebody with your username already persists on the server, please, choose another username.")),
            None => {
                return ConnectResult::Err(format!("FromPrimitive::from_i32() failed at [{}, {}]", file!(), line!()))
            }
        }

        // Ok.
        // Read info about all rooms.
        // Read room count.
        let mut room_count_buf = vec![0u8; std::mem::size_of::<u16>()];
        loop {
            match self.read_from_socket(&mut room_count_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => break,
                res => return ConnectResult::IoErr(res),
            }
        }
        let room_count = u16::decode::<u16>(&room_count_buf);
        if let Err(e) = room_count {
            return ConnectResult::Err(format!(
                "u64::decode::<u16>() failed, error: failed to decode on 'room_count_buf' (error: {}) at [{}, {}]",
                e, file!(), line!()
            ));
        }
        let room_count = room_count.unwrap();

        // Read rooms.
        for _ in 0..room_count {
            // Read room name len.
            let mut room_len_buf = vec![0u8; std::mem::size_of::<u8>()];
            loop {
                match self.read_from_socket(&mut room_len_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::IoErr(res),
                }
            }
            let room_len = u8::decode::<u8>(&room_len_buf);
            if let Err(e) = room_len {
                return ConnectResult::Err(format!(
                    "u16::decode::<u8>() failed, error: failed to decode on 'room_len_buf' (error: {}) at [{}, {}]",
                    e, file!(), line!()
                ));
            }
            let room_len = room_len.unwrap();

            // Read room.
            let mut room_buf = vec![0u8; room_len as usize];
            loop {
                match self.read_from_socket(&mut room_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::IoErr(res),
                }
            }
            let room_name = std::str::from_utf8(&room_buf);
            if let Err(e) = room_name {
                return ConnectResult::Err(
                    format!("std::str::from_utf8() failed, error: failed to convert on 'room_buf' (error: {}) at [{}, {}]",
                    e, file!(), line!()));
            }

            info_sender
                .send(ConnectInfo::RoomInfo(String::from(room_name.unwrap())))
                .unwrap();
        }

        // Read info about other users.
        // Read user count.
        let mut users_count_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut users_count_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => break,
                res => return ConnectResult::IoErr(res),
            }
        }

        let user_count = u64::decode::<u64>(&users_count_buf);
        if let Err(e) = user_count {
            return ConnectResult::Err(format!(
                "u64::decode::<u64>() failed, error: failed to decode on 'users_count_buf' (error: {}) at [{}, {}]",
                e, file!(), line!()
            ));
        }
        let user_count = user_count.unwrap();

        for _ in 0..user_count {
            // Read username len.
            let mut username_len_buf = vec![0u8; std::mem::size_of::<u16>()];
            loop {
                match self.read_from_socket(&mut username_len_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::IoErr(res),
                }
            }
            let username_len = u16::decode::<u16>(&username_len_buf);
            if let Err(e) = username_len {
                return ConnectResult::Err(format!(
                    "u16::decode::<u16>() failed, error: failed to decode on 'username_len_buf' (error: {}) at [{}, {}]",
                    e, file!(), line!()
                ));
            }
            let username_len = username_len.unwrap();

            // Read username.
            let mut username_buf = vec![0u8; username_len as usize];
            loop {
                match self.read_from_socket(&mut username_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::IoErr(res),
                }
            }
            let username = std::str::from_utf8(&username_buf);
            if let Err(e) = username {
                return ConnectResult::Err(
                    format!("std::str::from_utf8() failed, error: failed to convert on 'username_buf' (error: {}) at [{}, {}]",
                    e, file!(), line!()));
            }

            // Read room name len.
            let mut room_len_buf = vec![0u8; std::mem::size_of::<u8>()];
            loop {
                match self.read_from_socket(&mut room_len_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::IoErr(res),
                }
            }
            let room_len = u8::decode::<u8>(&room_len_buf);
            if let Err(e) = room_len {
                return ConnectResult::Err(format!(
                    "u16::decode::<u8>() failed, error: {} at [{}, {}]",
                    e,
                    file!(),
                    line!()
                ));
            }
            let room_len = room_len.unwrap();

            // Read room.
            let mut room_buf = vec![0u8; room_len as usize];
            loop {
                match self.read_from_socket(&mut room_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::IoErr(res),
                }
            }
            let room_name = std::str::from_utf8(&room_buf);
            if let Err(e) = room_name {
                return ConnectResult::Err(
                    format!("std::str::from_utf8() failed, error: failed to convert (error: {}) at [{}, {}]",
                    e, file!(), line!()));
            }

            // Read ping.
            let mut ping_buf = vec![0u8; std::mem::size_of::<u16>()];
            loop {
                match self.read_from_socket(&mut ping_buf) {
                    IoResult::WouldBlock => {
                        thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                        continue;
                    }
                    IoResult::Ok(_) => break,
                    res => return ConnectResult::IoErr(res),
                }
            }
            let ping_ms = u16::decode::<u16>(&ping_buf);
            if let Err(e) = ping_ms {
                return ConnectResult::Err(format!(
                    "u16::decode::<u16>() failed, error: {}, at [{}, {}]",
                    e,
                    file!(),
                    line!()
                ));
            }
            let ping_ms = ping_ms.unwrap();

            info_sender
                .send(ConnectInfo::UserInfo(
                    UserInfo::new(String::from(username.unwrap())),
                    String::from(room_name.unwrap()),
                    ping_ms,
                ))
                .unwrap();
        }

        info_sender.send(ConnectInfo::End).unwrap(); // End.

        self.user_state = UserState::Connected;

        ConnectResult::Ok
    }
    fn send_keep_alive_check(&mut self) -> Result<(), IoResult> {
        // Prepare data ID buffer.
        let data_id = ClientMessageTcp::KeepAliveCheck.to_u16();
        if data_id.is_none() {
            return Err(IoResult::Err(format!(
                "ClientMessage::KeepAliveCheck.to_u16() failed at [{}, {}]",
                file!(),
                line!()
            )));
        }
        let data_id = data_id.unwrap();
        let data_id_buf = u16::encode::<u16>(&data_id);
        if let Err(e) = data_id_buf {
            return Err(IoResult::Err(format!(
                "u16::encode::<u16>() failed on value {}, error: {} at [{}, {}]",
                data_id,
                e,
                file!(),
                line!()
            )));
        }
        let mut data_id_buf = data_id_buf.unwrap();

        loop {
            match self.write_to_socket(&mut data_id_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => {
                    break;
                }
                res => {
                    return Err(res);
                }
            }
        }

        Ok(())
    }
    fn read_u16_and_string_from_socket(&mut self) -> Result<String, IoResult> {
        let mut len_buf = vec![0u8; std::mem::size_of::<u16>()];
        loop {
            match self.read_from_socket(&mut len_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => break,
                IoResult::FIN => return Err(IoResult::FIN),
                IoResult::Err(msg) => {
                    return Err(IoResult::Err(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
            };
        }

        let len = u16::decode::<u16>(&len_buf);
        if let Err(e) = len {
            return Err(IoResult::Err(format!(
                "u16::decode::<u16>() failed, error: failed to decode (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let len = len.unwrap();

        let mut string_buf = vec![0u8; len as usize];
        loop {
            match self.read_from_socket(&mut string_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => break,
                IoResult::FIN => return Err(IoResult::FIN),
                IoResult::Err(msg) => {
                    return Err(IoResult::Err(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
            };
        }

        let string = String::from_utf8(string_buf);
        if let Err(e) = string {
            return Err(IoResult::Err(format!(
                "String::from_utf8() failed, error: failed to convert (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }

        Ok(string.unwrap())
    }
    fn read_u8_and_string_from_socket(&mut self) -> Result<String, IoResult> {
        let mut len_buf = vec![0u8; std::mem::size_of::<u8>()];
        loop {
            match self.read_from_socket(&mut len_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => break,
                IoResult::FIN => return Err(IoResult::FIN),
                IoResult::Err(msg) => {
                    return Err(IoResult::Err(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
            };
        }

        let len = u8::decode::<u8>(&len_buf);
        if let Err(e) = len {
            return Err(IoResult::Err(format!(
                "u16::decode::<u8>() failed, error: failed to decode (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let len = len.unwrap();

        let mut string_buf = vec![0u8; len as usize];
        loop {
            match self.read_from_socket(&mut string_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => break,
                IoResult::FIN => return Err(IoResult::FIN),
                IoResult::Err(msg) => {
                    return Err(IoResult::Err(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
            };
        }

        let string = String::from_utf8(string_buf);
        if let Err(e) = string {
            return Err(IoResult::Err(format!(
                "String::from_utf8() failed, error: failed to convert (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }

        Ok(string.unwrap())
    }
}
