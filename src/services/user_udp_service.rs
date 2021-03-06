// External.
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use druid::{ExtEventSink, Selector, Target};
use rand::RngCore;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

// Std.
use std::convert::TryInto;
use std::io::ErrorKind;
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use super::udp_packets::*;
use super::user_tcp_service::SECRET_KEY_SIZE;
use crate::global_params::*;
use crate::services::audio_service::audio_service::*;

pub const USER_UDP_SERVICE_UPDATE_USER_PING: Selector<UserPingInfo> =
    Selector::new("user_udp_servce_update_user_ping");

#[derive(Clone)]
pub struct UserPingInfo {
    pub username: String,
    pub ping_ms: u16,
    pub try_again_count: u8,
}

#[derive(Debug)]
pub struct UserUdpService {
    io_udp_mutex: Mutex<()>,
    udp_socket_copy: Option<UdpSocket>,
    username: String,
    pub secret_key: [u8; SECRET_KEY_SIZE],
}

impl UserUdpService {
    pub fn new() -> Self {
        UserUdpService {
            io_udp_mutex: Mutex::new(()),
            udp_socket_copy: None,
            username: String::from(""),
            secret_key: [0; SECRET_KEY_SIZE],
        }
    }
    pub fn assign_socket_and_name(&mut self, socket: UdpSocket, username: String) {
        self.udp_socket_copy = Some(socket);
        self.username = username;
    }
    pub fn send_voice_message(&mut self, voice_chunk: Vec<i16>) {
        let packet = ClientUdpMessage::VoiceMessage {
            samples: voice_chunk,
        };

        let binary_packet = bincode::serialize(&packet).unwrap();
        if binary_packet.len() + std::mem::size_of::<u16>() > UDP_PACKET_MAX_SIZE as usize {
            // using std::mem::size_of::<u16>() as packet size
            panic!(
                "Binary packet size + size_of::<u16> exceeded the limit ({}) at [{}, {}].",
                UDP_PACKET_MAX_SIZE,
                file!(),
                line!()
            );
        }

        // Encrypt.
        let mut rng = rand::thread_rng();
        let mut iv = [0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let mut encrypted_packet = Aes256CbcEnc::new(&self.secret_key.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&binary_packet);

        let packet_size: u16 = (encrypted_packet.len() + IV_LENGTH) as u16;
        let mut packet_size = bincode::serialize(&packet_size).unwrap();

        packet_size.append(&mut Vec::from(iv));
        packet_size.append(&mut encrypted_packet);

        // Send this buffer.
        if let Err(msg) = self.send(&self.udp_socket_copy.as_ref().unwrap(), &packet_size) {
            print!("{}, at [{}, {}]", msg, file!(), line!());
        }
    }
    pub fn connect(&mut self, udp_socket: &UdpSocket) -> Result<(), String> {
        let packet = ClientUdpMessage::Connect {
            username: self.username.clone(),
        };

        let mut binary_packet = bincode::serialize(&packet).unwrap();
        if binary_packet.len() + std::mem::size_of::<u16>() > UDP_PACKET_MAX_SIZE as usize {
            // using std::mem::size_of::<u16>() as packet size
            panic!(
                "Binary packet size + size_of::<u16> exceeded the limit ({}) at [{}, {}].",
                UDP_PACKET_MAX_SIZE,
                file!(),
                line!()
            );
        }
        let packet_size: u16 = binary_packet.len() as u16;
        let mut packet_size = bincode::serialize(&packet_size).unwrap();

        packet_size.append(&mut binary_packet);

        // Send this buffer.
        if let Err(msg) = self.send(udp_socket, &packet_size) {
            return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
        }

        // Wait for the first ping check.
        let mut recv_buffer = vec![0u8; UDP_PACKET_MAX_SIZE as usize];
        match self.recv(udp_socket, &mut recv_buffer) {
            Ok(byte_count) => {
                if byte_count < std::mem::size_of::<u16>() {
                    return Err(format!(
                        "received message is too small, at [{}, {}]",
                        file!(),
                        line!()
                    ));
                } else {
                    // Deserialize packet length.
                    let packet_len =
                        bincode::deserialize::<u16>(&recv_buffer[..std::mem::size_of::<u16>()]);
                    if let Err(e) = packet_len {
                        return Err(format!("{}, at [{}, {}]", e, file!(), line!()));
                    }
                    let packet_len = packet_len.unwrap();

                    // Check size.
                    if packet_len > UDP_PACKET_MAX_SIZE {
                        return Err(format!(
                            "received packet length is too big ({}/{}), at [{}, {}]",
                            packet_len,
                            UDP_PACKET_MAX_SIZE,
                            file!(),
                            line!()
                        ));
                    }

                    // Exclude size of the packet and trailing zeros.
                    recv_buffer = recv_buffer[std::mem::size_of::<u16>()..byte_count].to_vec();
                }
            }
            Err(msg) => {
                return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
            }
        }

        // Get IV.
        if recv_buffer.len() < IV_LENGTH {
            return Err(format!(
                "received data is too small, at [{}, {}].",
                file!(),
                line!()
            ));
        }
        let iv = recv_buffer[..IV_LENGTH].to_vec();
        recv_buffer = recv_buffer[IV_LENGTH..].to_vec();

        // Convert IV.
        let iv = iv.try_into();
        if iv.is_err() {
            return Err(format!(
                "failed to convert iv to generic array, at [{}, {}].",
                file!(),
                line!()
            ));
        }
        let iv: [u8; IV_LENGTH] = iv.unwrap();

        // Decrypt packet.
        let decrypted_packet = Aes256CbcDec::new(&self.secret_key.into(), &iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(&recv_buffer);
        if let Err(e) = decrypted_packet {
            return Err(format!("{:?}, at [{}, {}]", e, file!(), line!()));
        }
        let decrypted_packet = decrypted_packet.unwrap();

        // Deserialize.
        let packet_buf = bincode::deserialize::<ServerUdpMessage>(&decrypted_packet);
        if let Err(e) = packet_buf {
            return Err(format!("{:?}, at [{}, {}]", e, file!(), line!()));
        }
        let packet_buf = packet_buf.unwrap();

        match packet_buf {
            ServerUdpMessage::PingCheck => {}
            _ => {
                return Err(format!(
                    "unexpected packet type, at [{}, {}]",
                    file!(),
                    line!()
                ));
            }
        }

        // OK. Send it back.
        self.answer_ping(udp_socket)
    }
    pub fn handle_message(
        &mut self,
        udp_socket: &UdpSocket,
        event_sink: ExtEventSink,
        audio_service: Arc<Mutex<AudioService>>,
    ) -> Result<(), String> {
        let mut recv_buffer = vec![0u8; UDP_PACKET_MAX_SIZE as usize];
        match self.recv(udp_socket, &mut recv_buffer) {
            Ok(byte_count) => {
                if byte_count < std::mem::size_of::<u16>() {
                    return Err(format!(
                        "received message is too small, at [{}, {}]",
                        file!(),
                        line!()
                    ));
                } else {
                    // Deserialize packet length.
                    let packet_len =
                        bincode::deserialize::<u16>(&recv_buffer[..std::mem::size_of::<u16>()]);
                    if let Err(e) = packet_len {
                        return Err(format!("{}, at [{}, {}]", e, file!(), line!()));
                    }
                    let packet_len = packet_len.unwrap();

                    // Check size.
                    if packet_len > UDP_PACKET_MAX_SIZE {
                        return Err(format!(
                            "received packet length is too big ({}/{}), at [{}, {}]",
                            packet_len,
                            UDP_PACKET_MAX_SIZE,
                            file!(),
                            line!()
                        ));
                    }

                    // Exclude size of the packet and trailing zeros.
                    recv_buffer = recv_buffer[std::mem::size_of::<u16>()..byte_count].to_vec();
                }
            }
            Err(msg) => {
                return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
            }
        }

        // Get IV.
        if recv_buffer.len() < IV_LENGTH {
            return Err(format!(
                "received data is too small, at [{}, {}].",
                file!(),
                line!()
            ));
        }
        let iv = recv_buffer[..IV_LENGTH].to_vec();
        recv_buffer = recv_buffer[IV_LENGTH..].to_vec();

        // Convert IV.
        let iv = iv.try_into();
        if iv.is_err() {
            return Err(format!(
                "failed to convert iv to generic array, at [{}, {}].",
                file!(),
                line!()
            ));
        }
        let iv: [u8; IV_LENGTH] = iv.unwrap();

        // Decrypt packet.
        let decrypted_packet = Aes256CbcDec::new(&self.secret_key.into(), &iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(&recv_buffer);
        if let Err(e) = decrypted_packet {
            return Err(format!("{:?}, at [{}, {}]", e, file!(), line!()));
        }
        let decrypted_packet = decrypted_packet.unwrap();

        // Deserialize.
        let packet_buf = bincode::deserialize::<ServerUdpMessage>(&decrypted_packet);
        if let Err(e) = packet_buf {
            return Err(format!("{:?}, at [{}, {}]", e, file!(), line!()));
        }
        let packet_buf = packet_buf.unwrap();

        match packet_buf {
            ServerUdpMessage::PingCheck => {
                // Send it back.
                return self.answer_ping(udp_socket);
            }
            ServerUdpMessage::UserPing { username, ping_ms } => {
                event_sink
                    .submit_command(
                        USER_UDP_SERVICE_UPDATE_USER_PING,
                        UserPingInfo {
                            username,
                            ping_ms,
                            try_again_count: USER_CONNECT_FIRST_UDP_PING_RETRY_MAX_COUNT,
                        },
                        Target::Auto,
                    )
                    .expect("failed to submit USER_UDP_SERVICE_UPDATE_USER_PING command");
            }
            ServerUdpMessage::VoiceMessage { username, samples } => {
                audio_service
                    .lock()
                    .unwrap()
                    .add_user_voice_chunk(username, samples, event_sink);
            }
        }

        Ok(())
    }
    fn answer_ping(&self, udp_socket: &UdpSocket) -> Result<(), String> {
        let packet = ClientUdpMessage::PingCheck {};

        let binary_packet = bincode::serialize(&packet).unwrap();
        if binary_packet.len() + std::mem::size_of::<u16>() > UDP_PACKET_MAX_SIZE as usize {
            // using std::mem::size_of::<u16>() as packet size
            panic!(
                "Binary packet size + size_of::<u16> exceeded the limit ({}) at [{}, {}].",
                UDP_PACKET_MAX_SIZE,
                file!(),
                line!()
            );
        }

        // Encrypt.
        let mut rng = rand::thread_rng();
        let mut iv = [0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let mut encrypted_packet = Aes256CbcEnc::new(&self.secret_key.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&binary_packet);

        let packet_size: u16 = (encrypted_packet.len() + IV_LENGTH) as u16;
        let mut packet_size = bincode::serialize(&packet_size).unwrap();

        packet_size.append(&mut Vec::from(iv));
        packet_size.append(&mut encrypted_packet);

        // Send this buffer.
        if let Err(msg) = self.send(udp_socket, &packet_size) {
            return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn send(&self, udp_socket: &UdpSocket, buf: &[u8]) -> Result<(), String> {
        let _io_guard = self.io_udp_mutex.lock().unwrap();

        loop {
            match udp_socket.send(buf) {
                Ok(n) => {
                    if n != buf.len() {
                        return Err(format!("udp_socket.send() failed, error: sent only {} bytes out of {}, at [{}, {}]",
                        n, buf.len(), file!(), line!()));
                    } else {
                        break;
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_UDP_MESSAGE_MS));
                    continue;
                }
                Err(e) => {
                    return Err(format!(
                        "udp_socket.send() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ));
                }
            }
        }

        Ok(())
    }
    pub fn peek(&self, udp_socket: &UdpSocket, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        #[cfg(target_os = "linux")]
        {
            match udp_socket.peek(buf) {
                Ok(n) => {
                    return Ok(n);
                }
                Err(e) => return Err(e),
            }
        }
        #[cfg(target_os = "windows")]
        {
            // windows returns error 10040 - WSAEMSGSIZE (https://docs.microsoft.com/en-us/troubleshoot/windows-server/networking/wsaemsgsize-error-10040-in-winsock-2)
            // if the 'buf' is smaller than incoming packet
            let mut bigger_buf = vec![0u8; UDP_PACKET_MAX_SIZE as usize];
            match udp_socket.peek(&mut bigger_buf) {
                Ok(n) => {
                    for i in 0..n {
                        if i < buf.len() {
                            buf[i] = bigger_buf[i];
                        } else {
                            break;
                        }
                    }
                    return Ok(buf.len());
                }
                Err(e) => return Err(e),
            }
        }
    }
    pub fn recv(&mut self, udp_socket: &UdpSocket, buf: &mut [u8]) -> Result<usize, String> {
        let _io_guard = self.io_udp_mutex.lock().unwrap();

        loop {
            match udp_socket.recv(buf) {
                Ok(byte_count) => return Ok(byte_count),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_UDP_MESSAGE_MS));
                    continue;
                }
                Err(e) => {
                    return Err(format!(
                        "udp_socket.recv() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ));
                }
            }
        }
    }
}
