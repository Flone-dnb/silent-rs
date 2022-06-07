// External.
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::Aes256;
use cmac::{Cmac, Mac};
use druid::{ExtEventSink, Selector, Target};
use num_bigint::{BigUint, RandomBits};
use rand::{Rng, RngCore};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub const SECRET_KEY_SIZE: usize = 32;

// Std.
use std::convert::TryInto;
use std::io::prelude::*;
use std::net::*;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// Custom.
use super::tcp_packets::*;
use crate::global_params::*;

const A_B_BITS: u64 = 2048;

pub const USER_TCP_SERVICE_USER_CONNECTED: Selector<String> =
    Selector::new("user_tcp_service_user_connected");

pub const USER_TCP_SERVICE_USER_DISCONNECTED: Selector<String> =
    Selector::new("user_tcp_service_user_disconnected");

pub const USER_TCP_SERVICE_USER_MESSAGE: Selector<UserMessageInfo> =
    Selector::new("user_tcp_service_user_message");

pub const USER_TCP_SERVICE_MOVE_USER_TO_ROOM: Selector<UserMoveInfo> =
    Selector::new("user_tcp_service_move_user_to_room");

#[derive(Debug)]
pub enum UserState {
    NotConnected,
    Connected,
}

pub struct UserMessageInfo {
    pub username: String,
    pub message: String,
}

pub struct UserMoveInfo {
    pub username: String,
    pub room_to: String,
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
    ErrServerOffline,
    ErrServerIsFull,
    UsernameTaken,
    SleepWithErr(usize), // sleep time in sec.
    WrongProtocol(u64),  // needed protocol
    Err(String),
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
    pub secret_key: [u8; SECRET_KEY_SIZE],
}

impl UserTcpService {
    pub fn new(server_password: String) -> Self {
        UserTcpService {
            user_state: UserState::NotConnected,
            tcp_socket: None,
            server_password,
            user_info: UserInfo {
                username: String::from(""),
            },
            io_tcp_mutex: Mutex::new(()),
            secret_key: [0; SECRET_KEY_SIZE],
        }
    }
    pub fn establish_secure_connection(&mut self) -> Result<Vec<u8>, HandleMessageResult> {
        // Generate secret key 'b'.
        let mut rng = rand::thread_rng();
        let b: BigUint = rng.sample(RandomBits::new(A_B_BITS));

        // Receive 2 values: p (BigUint), g (BigUint) values.
        // Get 'p' len.
        let mut p_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut p_len_buf) {
                IoResult::FIN => {
                    return Err(HandleMessageResult::IOError(IoResult::FIN));
                }
                IoResult::Err(msg) => {
                    return Err(HandleMessageResult::OtherErr(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }
        let p_len = bincode::deserialize::<u64>(&p_len_buf);
        if let Err(e) = p_len {
            return Err(HandleMessageResult::OtherErr(format!(
                "bincode::deserialize failed, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let p_len = p_len.unwrap();

        // Get 'p' value.
        let mut p_buf = vec![0u8; p_len as usize];
        loop {
            match self.read_from_socket(&mut p_buf) {
                IoResult::FIN => {
                    return Err(HandleMessageResult::IOError(IoResult::FIN));
                }
                IoResult::Err(msg) => {
                    return Err(HandleMessageResult::OtherErr(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }
        let p_buf = bincode::deserialize::<BigUint>(&p_buf);
        if let Err(e) = p_buf {
            return Err(HandleMessageResult::OtherErr(format!(
                "bincode::deserialize failed, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let p = p_buf.unwrap();

        // Get 'g' len.
        let mut g_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut g_len_buf) {
                IoResult::FIN => {
                    return Err(HandleMessageResult::IOError(IoResult::FIN));
                }
                IoResult::Err(msg) => {
                    return Err(HandleMessageResult::OtherErr(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }
        let g_len = bincode::deserialize::<u64>(&g_len_buf);
        if let Err(e) = g_len {
            return Err(HandleMessageResult::OtherErr(format!(
                "bincode::deserialize failed, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let g_len = g_len.unwrap();

        // Get 'g' value.
        let mut g_buf = vec![0u8; g_len as usize];
        loop {
            match self.read_from_socket(&mut g_buf) {
                IoResult::FIN => {
                    return Err(HandleMessageResult::IOError(IoResult::FIN));
                }
                IoResult::Err(msg) => {
                    return Err(HandleMessageResult::OtherErr(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }
        let g_buf = bincode::deserialize::<BigUint>(&g_buf);
        if let Err(e) = g_buf {
            return Err(HandleMessageResult::OtherErr(format!(
                "bincode::deserialize failed, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let g = g_buf.unwrap();

        // Calculate the open key B.
        let b_open = g.modpow(&b, &p);

        // Receive the open key A size.
        let mut a_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut a_open_len_buf) {
                IoResult::FIN => {
                    return Err(HandleMessageResult::IOError(IoResult::FIN));
                }
                IoResult::Err(msg) => {
                    return Err(HandleMessageResult::OtherErr(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        let a_open_len = bincode::deserialize::<u64>(&a_open_len_buf);
        if let Err(e) = a_open_len {
            return Err(HandleMessageResult::OtherErr(format!(
                "bincode::deserialize failed, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let a_open_len = a_open_len.unwrap();

        // Receive the open key A.
        let mut a_open_buf = vec![0u8; a_open_len as usize];
        loop {
            match self.read_from_socket(&mut a_open_buf) {
                IoResult::FIN => {
                    return Err(HandleMessageResult::IOError(IoResult::FIN));
                }
                IoResult::Err(msg) => {
                    return Err(HandleMessageResult::OtherErr(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        let a_open_big = bincode::deserialize::<BigUint>(&a_open_buf);
        if let Err(e) = a_open_big {
            return Err(HandleMessageResult::OtherErr(format!(
                "bincode::deserialize failed, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let a_open_big = a_open_big.unwrap();

        // Prepare to send open key B.
        let mut b_open_buf = bincode::serialize(&b_open).unwrap();

        // Send open key 'B'.
        let b_open_len = b_open_buf.len() as u64;
        let b_open_len_buf = bincode::serialize(&b_open_len);
        if let Err(e) = b_open_len_buf {
            return Err(HandleMessageResult::OtherErr(format!(
                "bincode::serialize failed, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let mut b_open_len_buf = b_open_len_buf.unwrap();
        b_open_len_buf.append(&mut b_open_buf);
        loop {
            match self.write_to_socket(&mut b_open_len_buf) {
                IoResult::FIN => {
                    return Err(HandleMessageResult::IOError(IoResult::FIN));
                }
                IoResult::Err(msg) => {
                    return Err(HandleMessageResult::OtherErr(format!(
                        "{} at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    )));
                }
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        // Calculate the secret key.
        let secret_key = a_open_big.modpow(&b, &p);
        let mut secret_key_str = secret_key.to_str_radix(10);

        let key_length = 32;

        if secret_key_str.len() < key_length {
            if secret_key_str.is_empty() {
                return Err(HandleMessageResult::OtherErr(format!(
                    "generated secret key is empty, at [{}, {}].",
                    file!(),
                    line!()
                )));
            }

            loop {
                secret_key_str += &secret_key_str.clone();

                if secret_key_str.len() >= key_length {
                    break;
                }
            }
        }

        Ok(Vec::from(&secret_key_str[0..key_length]))
    }
    pub fn enter_room(&mut self, room: &str) -> HandleMessageResult {
        if self.tcp_socket.is_none() {
            return HandleMessageResult::OtherErr(format!(
                "UserTcpService::send_user_text_message() failed, error: tcp_socket was None at [{}, {}]", file!(), line!()
            ));
        }

        let client_packet = ClientTcpMessage::UserEnterRoom {
            room_name: String::from(room),
        };

        // Serialize packet.
        let binary_client_packet = bincode::serialize(&client_packet);
        if let Err(e) = binary_client_packet {
            return HandleMessageResult::OtherErr(format!(
                "bincode::serialize failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        let mut binary_client_packet = binary_client_packet.unwrap();

        // CMAC.
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
        mac.update(&binary_client_packet);
        let result = mac.finalize();
        let mut tag_bytes = result.into_bytes().to_vec();
        if tag_bytes.len() != CMAC_TAG_LENGTH {
            return HandleMessageResult::OtherErr(format!(
                "unexpected tag length: {} != {} at [{}, {}]",
                tag_bytes.len(),
                CMAC_TAG_LENGTH,
                file!(),
                line!()
            ));
        }

        binary_client_packet.append(&mut tag_bytes);

        // Encrypt packet.
        let mut rng = rand::thread_rng();
        let mut iv = [0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let mut encrypted_packet = Aes256CbcEnc::new(&self.secret_key.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&binary_client_packet);

        // Prepare encrypted packet len buffer.
        let encrypted_len = (encrypted_packet.len() + IV_LENGTH) as u16;
        let encrypted_len_buf = bincode::serialize(&encrypted_len);
        if let Err(e) = encrypted_len_buf {
            return HandleMessageResult::OtherErr(format!(
                "bincode::serialize failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        let mut send_buffer = encrypted_len_buf.unwrap();

        // Merge all to one buffer.
        send_buffer.append(&mut Vec::from(iv));
        send_buffer.append(&mut encrypted_packet);

        // Send to server.
        loop {
            match self.write_to_socket(&mut send_buffer) {
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
    pub fn send_user_text_message(&mut self, message: String) -> HandleMessageResult {
        if self.tcp_socket.is_none() {
            return HandleMessageResult::OtherErr(format!(
                "tcp_socket was None at [{}, {}]",
                file!(),
                line!()
            ));
        }

        let client_message_packet = ClientTcpMessage::UserMessage { message };

        // Serialize packet.
        let binary_client_message_packet = bincode::serialize(&client_message_packet);
        if let Err(e) = binary_client_message_packet {
            return HandleMessageResult::OtherErr(format!(
                "bincode::serialize failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        let mut binary_client_message_packet = binary_client_message_packet.unwrap();

        // CMAC.
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
        mac.update(&binary_client_message_packet);
        let result = mac.finalize();
        let mut tag_bytes = result.into_bytes().to_vec();
        if tag_bytes.len() != CMAC_TAG_LENGTH {
            return HandleMessageResult::OtherErr(format!(
                "unexpected tag length: {} != {} at [{}, {}]",
                tag_bytes.len(),
                CMAC_TAG_LENGTH,
                file!(),
                line!()
            ));
        }

        binary_client_message_packet.append(&mut tag_bytes);

        // Encrypt packet.
        let mut rng = rand::thread_rng();
        let mut iv = [0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let mut encrypted_message_packet =
            Aes256CbcEnc::new(&self.secret_key.into(), &iv.into())
                .encrypt_padded_vec_mut::<Pkcs7>(&binary_client_message_packet);

        // Prepare encrypted packet len buffer.
        let encrypted_message_len = (encrypted_message_packet.len() + IV_LENGTH) as u16;
        let encrypted_message_len_buf = bincode::serialize(&encrypted_message_len);
        if let Err(e) = encrypted_message_len_buf {
            return HandleMessageResult::OtherErr(format!(
                "bincode::serialize failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        let mut encrypted_message_len_buf = encrypted_message_len_buf.unwrap();

        // Merge all to one buffer.
        let mut out_buffer: Vec<u8> = Vec::new();
        out_buffer.append(&mut encrypted_message_len_buf);
        out_buffer.append(&mut Vec::from(iv));
        out_buffer.append(&mut encrypted_message_packet);

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
                "UserTcpService::read_from_socket_tcp() failed, error: tcp_socket was None at [{}, {}]",
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
                "UserTcpService::write_to_socket_tcp() failed, error: tcp_socket was None at [{}, {}]",
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
        message_size: u16,
        event_sink: ExtEventSink,
    ) -> HandleMessageResult {
        // Receive packet.
        let mut packet_buf = vec![0u8; message_size as usize];
        loop {
            match self.read_from_socket(&mut packet_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return HandleMessageResult::IOError(res),
            }
        }

        // Get IV.
        if packet_buf.len() < IV_LENGTH {
            return HandleMessageResult::IOError(IoResult::Err(format!(
                "received data is too small, at [{}, {}].",
                file!(),
                line!()
            )));
        }
        let iv = packet_buf[..IV_LENGTH].to_vec();
        packet_buf = packet_buf[IV_LENGTH..].to_vec();

        // Convert IV.
        let iv = iv.try_into();
        if iv.is_err() {
            return HandleMessageResult::OtherErr(format!(
                "failed to convert iv to generic array, at [{}, {}].",
                file!(),
                line!()
            ));
        }
        let iv: [u8; IV_LENGTH] = iv.unwrap();

        // Decrypt packet.
        let binary_server_packet = Aes256CbcDec::new(&self.secret_key.into(), &iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(&packet_buf);
        if let Err(e) = binary_server_packet {
            return HandleMessageResult::IOError(IoResult::Err(format!(
                "unable to decrypt a packet (error: {}), at [{}, {}].",
                e,
                file!(),
                line!()
            )));
        }
        let mut binary_server_packet = binary_server_packet.unwrap();

        // CMAC
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
        let tag: Vec<u8> = binary_server_packet
            .drain(binary_server_packet.len().saturating_sub(CMAC_TAG_LENGTH)..)
            .collect();
        mac.update(&binary_server_packet);

        // Convert tag.
        let tag = tag.try_into();
        if tag.is_err() {
            return HandleMessageResult::OtherErr(format!(
                "failed to convert cmac tag to generic array, at [{}, {}]",
                file!(),
                line!()
            ));
        }
        let tag: [u8; CMAC_TAG_LENGTH] = tag.unwrap();

        if let Err(e) = mac.verify(&tag.into()) {
            return HandleMessageResult::OtherErr(format!(
                "Incorrect tag (error: {}), at [{}, {}].",
                e,
                file!(),
                line!()
            ));
        }

        // Deserialize.
        let server_packet = bincode::deserialize::<ServerTcpMessage>(&binary_server_packet);
        if let Err(e) = server_packet {
            return HandleMessageResult::IOError(IoResult::Err(format!(
                "Unable to deserialize a packet (error: {}), at [{}, {}].",
                e,
                file!(),
                line!()
            )));
        }
        let server_packet = server_packet.unwrap();

        match server_packet {
            ServerTcpMessage::KeepAliveCheck => {
                if let Err(e) = self.send_keep_alive_check() {
                    return HandleMessageResult::IOError(e);
                } else {
                    return HandleMessageResult::Ok;
                }
            }
            ServerTcpMessage::UserConnected { username } => {
                event_sink
                    .submit_command(USER_TCP_SERVICE_USER_CONNECTED, username, Target::Auto)
                    .expect("failed to submit USER_TCP_SERVICE_USER_CONNECTED command");
            }
            ServerTcpMessage::UserDisconnected { username } => {
                event_sink
                    .submit_command(USER_TCP_SERVICE_USER_DISCONNECTED, username, Target::Auto)
                    .expect("failed to submit USER_TCP_SERVICE_USER_DISCONNECTED command");
            }
            ServerTcpMessage::UserMessage { username, message } => {
                event_sink
                    .submit_command(
                        USER_TCP_SERVICE_USER_MESSAGE,
                        UserMessageInfo { username, message },
                        Target::Auto,
                    )
                    .expect("failed to submit USER_TCP_SERVICE_USER_MESSAGE command");
            }
            ServerTcpMessage::UserEntersRoom {
                username,
                room_enters,
            } => {
                event_sink
                    .submit_command(
                        USER_TCP_SERVICE_MOVE_USER_TO_ROOM,
                        UserMoveInfo {
                            username,
                            room_to: room_enters,
                        },
                        Target::Auto,
                    )
                    .expect("failed to submit USER_TCP_SERVICE_MOVE_USER_TO_ROOM command");
            }
        }

        HandleMessageResult::Ok
    }

    pub fn connect_user(
        &mut self,
        info_sender: std::sync::mpsc::Sender<ConnectInfo>,
    ) -> ConnectResult {
        let packet = ClientConnectPacket {
            net_protocol_version: NETWORK_PROTOCOL_VERSION,
            username: self.user_info.username.clone(),
            password: self.server_password.clone(),
        };

        let mut binary_packet = bincode::serialize(&packet).unwrap();

        // CMAC.
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
        mac.update(&binary_packet);
        let result = mac.finalize();
        let mut tag_bytes = result.into_bytes().to_vec();
        if tag_bytes.len() != CMAC_TAG_LENGTH {
            return ConnectResult::Err(format!(
                "unexpected tag length: {} != {} at [{}, {}]",
                tag_bytes.len(),
                CMAC_TAG_LENGTH,
                file!(),
                line!()
            ));
        }

        binary_packet.append(&mut tag_bytes);

        // Encrypt binary packet.
        let mut rng = rand::thread_rng();
        let mut iv = [0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let mut encrypted_binary_packet = Aes256CbcEnc::new(&self.secret_key.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&binary_packet);

        if encrypted_binary_packet.len() + IV_LENGTH + std::mem::size_of::<u16>()
            > std::u16::MAX as usize
        {
            // should never happen
            // using std::mem::size_of::<u16>() as packet size
            panic!(
                "Encrypted binary packet size + size_of::<u16> exceeded u16::MAX at [{}, {}].",
                file!(),
                line!()
            );
        }
        let packet_size: u16 = (encrypted_binary_packet.len() + IV_LENGTH) as u16;
        let mut packet_size = bincode::serialize(&packet_size).unwrap();

        let mut send_buffer: Vec<u8> = Vec::new();
        send_buffer.append(&mut packet_size);
        send_buffer.append(&mut Vec::from(iv));
        send_buffer.append(&mut encrypted_binary_packet);

        // Send this buffer.
        loop {
            match self.write_to_socket(&mut send_buffer) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                }
                IoResult::Ok(_bytes) => break,
                res => return ConnectResult::IoErr(res),
            };
        }

        // Wait for answer.
        // We usually use 'u16' as size of the data
        // but this "packet" is an exception.
        let mut data_size_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut data_size_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return ConnectResult::IoErr(res),
            }
        }

        let data_size = bincode::deserialize::<u64>(&data_size_buf).unwrap();
        if data_size > TCP_CONNECT_ANSWER_PACKET_MAX_SIZE {
            return ConnectResult::IoErr(IoResult::Err(format!("The data size received from the server ({}) exceeds the maximum ({}), at [{}, {}].",
            data_size, TCP_CONNECT_ANSWER_PACKET_MAX_SIZE, file!(), line!())));
        }

        let mut data_buf = vec![0u8; data_size as usize];
        loop {
            match self.read_from_socket(&mut data_buf) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return ConnectResult::IoErr(res),
            }
        }

        // Get IV.
        if data_buf.len() < IV_LENGTH {
            return ConnectResult::Err(format!(
                "received data is too small, at [{}, {}].",
                file!(),
                line!()
            ));
        }
        let iv = data_buf[..IV_LENGTH].to_vec();
        data_buf = data_buf[IV_LENGTH..].to_vec();

        // Convert IV.
        let iv = iv.try_into();
        if iv.is_err() {
            return ConnectResult::Err(format!(
                "failed to convert iv to generic array, at [{}, {}].",
                file!(),
                line!()
            ));
        }
        let iv: [u8; IV_LENGTH] = iv.unwrap();

        // Decrypt packet.
        let binary_server_connect_packet = Aes256CbcDec::new(&self.secret_key.into(), &iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(&data_buf);
        if let Err(e) = binary_server_connect_packet {
            return ConnectResult::IoErr(IoResult::Err(format!(
                "Unable to decrypt a packet (error: {}), at [{}, {}].",
                e,
                file!(),
                line!()
            )));
        }
        let mut binary_server_connect_packet = binary_server_connect_packet.unwrap();

        // CMAC
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
        let tag: Vec<u8> = binary_server_connect_packet
            .drain(
                binary_server_connect_packet
                    .len()
                    .saturating_sub(CMAC_TAG_LENGTH)..,
            )
            .collect();
        mac.update(&binary_server_connect_packet);

        // Convert tag.
        let tag = tag.try_into();
        if tag.is_err() {
            return ConnectResult::Err(format!(
                "failed to convert cmac tag to generic array, at [{}, {}]",
                file!(),
                line!()
            ));
        }
        let tag: [u8; CMAC_TAG_LENGTH] = tag.unwrap();

        if let Err(e) = mac.verify(&tag.into()) {
            return ConnectResult::Err(format!(
                "Incorrect tag (error: {}), at [{}, {}].",
                e,
                file!(),
                line!()
            ));
        }

        // Deserialize.
        let server_connect_packet =
            bincode::deserialize::<ServerTcpConnectPacket>(&binary_server_connect_packet);
        if let Err(e) = server_connect_packet {
            return ConnectResult::IoErr(IoResult::Err(format!(
                "Unable to deserialize a packet (error: {}), at [{}, {}].",
                e,
                file!(),
                line!()
            )));
        }
        let server_connect_packet = server_connect_packet.unwrap();

        // See answer.
        match server_connect_packet.answer {
            ConnectServerAnswer::Ok => {}
            ConnectServerAnswer::WrongPassword => {
                return ConnectResult::SleepWithErr(PASSWORD_RETRY_DELAY_SEC);
            }
            ConnectServerAnswer::WrongVersion => {
                return ConnectResult::WrongProtocol(
                    server_connect_packet.correct_net_protocol.unwrap(),
                );
            }
            ConnectServerAnswer::UsernameTaken => return ConnectResult::UsernameTaken,
            ConnectServerAnswer::ServerIsFull => return ConnectResult::ErrServerIsFull,
        }

        // Read info about all rooms and users.
        for room_info in server_connect_packet
            .connected_info
            .as_ref()
            .unwrap()
            .iter()
        {
            info_sender
                .send(ConnectInfo::RoomInfo(room_info.room_name.clone()))
                .unwrap();

            for user in room_info.users.iter() {
                info_sender
                    .send(ConnectInfo::UserInfo(
                        UserInfo::new(user.username.clone()),
                        String::from(room_info.room_name.clone()),
                        user.ping,
                    ))
                    .unwrap();
            }
        }

        info_sender.send(ConnectInfo::End).unwrap(); // End.

        self.user_state = UserState::Connected;

        ConnectResult::Ok
    }
    fn send_keep_alive_check(&mut self) -> Result<(), IoResult> {
        let client_packet = ClientTcpMessage::KeepAliveCheck;

        // Serialize packet.
        let binary_client_packet = bincode::serialize(&client_packet);
        if let Err(e) = binary_client_packet {
            return Err(IoResult::Err(format!(
                "bincode::serialize failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let mut binary_client_packet = binary_client_packet.unwrap();

        // CMAC.
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
        mac.update(&binary_client_packet);
        let result = mac.finalize();
        let mut tag_bytes = result.into_bytes().to_vec();
        if tag_bytes.len() != CMAC_TAG_LENGTH {
            return Err(IoResult::Err(format!(
                "unexpected tag length: {} != {} at [{}, {}]",
                tag_bytes.len(),
                CMAC_TAG_LENGTH,
                file!(),
                line!()
            )));
        }

        binary_client_packet.append(&mut tag_bytes);

        // Encrypt packet.
        let mut rng = rand::thread_rng();
        let mut iv = [0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let mut encrypted_packet = Aes256CbcEnc::new(&self.secret_key.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&binary_client_packet);

        // Prepare encrypted packet len buffer.
        let encrypted_packet_len = (encrypted_packet.len() + IV_LENGTH) as u16;
        let encrypted_packet_len_buf = bincode::serialize(&encrypted_packet_len);
        if let Err(e) = encrypted_packet_len_buf {
            return Err(IoResult::Err(format!(
                "bincode::serialize failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            )));
        }
        let mut encrypted_packet_len_buf = encrypted_packet_len_buf.unwrap();

        // Merge all to one buffer.
        let mut out_buffer: Vec<u8> = Vec::new();
        out_buffer.append(&mut encrypted_packet_len_buf);
        out_buffer.append(&mut Vec::from(iv));
        out_buffer.append(&mut encrypted_packet);

        // Send to server.
        loop {
            match self.write_to_socket(&mut out_buffer) {
                IoResult::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_TCP_MESSAGE_MS));
                    continue;
                }
                IoResult::Ok(_bytes) => break,
                res => return Err(res),
            }
        }

        Ok(())
    }
}
