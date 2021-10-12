// External.
use aes::Aes128;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Ecb};
use bytevec::ByteEncodable;
use druid::{ExtEventSink, Selector, Target};
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::cast::ToPrimitive;

// Std.
use std::io::ErrorKind;
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use super::udp_packets::*;
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

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum ServerMessageUdp {
    UserPing = 0,
    PingCheck = 1,
    VoiceMessage = 2,
}

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum ClientMessageUdp {
    VoicePacket = 0,
    PingCheck = 1,
}

#[derive(Debug)]
pub struct UserUdpService {
    io_udp_mutex: Mutex<()>,
    udp_socket_copy: Option<UdpSocket>,
    username: String,
    pub secret_key: Vec<u8>,
}

impl UserUdpService {
    pub fn new() -> Self {
        UserUdpService {
            io_udp_mutex: Mutex::new(()),
            udp_socket_copy: None,
            username: String::from(""),
            secret_key: Vec::new(),
        }
    }
    pub fn assign_socket_and_name(&mut self, socket: UdpSocket, username: String) {
        self.udp_socket_copy = Some(socket);
        self.username = username;
    }
    pub fn send_voice_message(&mut self, voice_chunk: Vec<i16>) {
        // prepare voice packet:
        // (u8) - packet type (ClientMessageUdp::VoicePacket)
        // (u16) - voice data (encrypted) size in bytes
        // (size) - voice data (encrypted)

        let packet_id = ClientMessageUdp::VoicePacket.to_u8().unwrap();

        // Convert voice_chunk from Vec<i16> to Vec<u8>
        let mut voice_data: Vec<u8> = Vec::new();
        for val in voice_chunk.into_iter() {
            let mut _val_u: u16 = 0;
            unsafe {
                _val_u = std::mem::transmute::<i16, u16>(val);
            }
            let res = u16::encode::<u16>(&_val_u);
            if let Err(e) = res {
                panic!(
                    "An error occurred, error: {}, at [{}, {}]",
                    e,
                    file!(),
                    line!()
                );
            }

            let mut vec = res.unwrap();

            voice_data.append(&mut vec);
        }

        // Encrypt voice data.
        type Aes128Ecb = Ecb<Aes128, Pkcs7>;
        let cipher = Aes128Ecb::new_from_slices(&self.secret_key, Default::default()).unwrap();
        let mut encrypted_message = cipher.encrypt_vec(&voice_data);

        // voice data (encrypted) len
        let encrypted_voice_data_len: u16 = encrypted_message.len() as u16;

        let encrypted_voice_data_len_buf = u16::encode::<u16>(&encrypted_voice_data_len);
        if let Err(e) = encrypted_voice_data_len_buf {
            panic!(
                "An error occurred, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            );
        }
        let mut encrypted_voice_data_len_buf = encrypted_voice_data_len_buf.unwrap();

        let mut out_buf: Vec<u8> = Vec::new();
        out_buf.push(packet_id);
        out_buf.append(&mut encrypted_voice_data_len_buf);
        out_buf.append(&mut encrypted_message);

        match self.send(self.udp_socket_copy.as_ref().unwrap(), &out_buf) {
            Err(msg) => {
                panic!("{}, at [{}, {}]", msg, file!(), line!());
            }
            _ => {}
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

        // Decrypt.
        type Aes128Ecb = Ecb<Aes128, Pkcs7>;
        let cipher = Aes128Ecb::new_from_slices(&self.secret_key, Default::default()).unwrap();
        let decrypted_packet = cipher.decrypt_vec(&recv_buffer);
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
        let packet_size: u16 = recv_buffer.len() as u16;
        let mut packet_size = bincode::serialize(&packet_size).unwrap();

        packet_size.append(&mut recv_buffer);

        if let Err(msg) = self.send(udp_socket, &packet_size) {
            return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
        }

        Ok(())
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

        // Decrypt.
        type Aes128Ecb = Ecb<Aes128, Pkcs7>;
        let cipher = Aes128Ecb::new_from_slices(&self.secret_key, Default::default()).unwrap();
        let decrypted_packet = cipher.decrypt_vec(&recv_buffer);
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
        }

        // match FromPrimitive::from_u8(buf[0]) {
        //     Some(ServerMessageUdp::VoiceMessage) => {
        //         // Packet structure:
        //         // (u8) - id (ServerMessageUdp::VoiceMessage)
        //         // (u8) - username len
        //         // (size) - username
        //         // (u16) - voice data (encrypted) len
        //         // (size) - voice data (encrypted)

        //         let username_len = buf[1];
        //         let mut _read_i = 2usize;
        //         let username = String::from_utf8(Vec::from(&buf[2..2 + username_len as usize]));
        //         _read_i += username_len as usize;
        //         if let Err(e) = username {
        //             return Err(format!(
        //                 "String::from_utf8() failed, error: {}, at [{}, {}]",
        //                 e,
        //                 file!(),
        //                 line!()
        //             ));
        //         }
        //         let username = username.unwrap();

        //         // Read voice data (encrypted) len.
        //         let encrypted_voice_data_len_buf =
        //             &buf[_read_i.._read_i + std::mem::size_of::<u16>()];
        //         _read_i += std::mem::size_of::<u16>();

        //         let encrypted_voice_data_len = u16::decode::<u16>(&encrypted_voice_data_len_buf);
        //         if let Err(e) = encrypted_voice_data_len {
        //             return Err(format!(
        //                 "u16::decode::<u16>() failed, error: {}, at [{}, {}]",
        //                 e,
        //                 file!(),
        //                 line!()
        //             ));
        //         }
        //         let encrypted_voice_data_len = encrypted_voice_data_len.unwrap();

        //         // Read voice data (encrypted)
        //         let encrypted_voice_data =
        //             &buf[_read_i.._read_i + encrypted_voice_data_len as usize];

        //         // Decrypt voice data.
        //         type Aes128Ecb = Ecb<Aes128, Pkcs7>;
        //         let cipher =
        //             Aes128Ecb::new_from_slices(&self.secret_key, Default::default()).unwrap();
        //         let decrypted_message = cipher.decrypt_vec(encrypted_voice_data);
        //         if let Err(e) = decrypted_message {
        //             return Err(format!(
        //                 "cipher.decrypt_vec() failed, error: {}, at [{}, {}]",
        //                 e,
        //                 file!(),
        //                 line!()
        //             ));
        //         }
        //         let user_voice_message = decrypted_message.unwrap();

        //         let mut voice_data_vec: Vec<i16> = Vec::new();

        //         for i in (0..user_voice_message.len()).step_by(std::mem::size_of::<i16>()) {
        //             let val_u =
        //                 u16::decode::<u16>(&user_voice_message[i..i + std::mem::size_of::<i16>()]);
        //             if let Err(e) = val_u {
        //                 return Err(format!(
        //                     "u16::decode::<u16>() failed, error: {}, at [{}, {}]",
        //                     e,
        //                     file!(),
        //                     line!()
        //                 ));
        //             }
        //             let val_u = val_u.unwrap();

        //             let mut _val: i16 = 0;
        //             unsafe {
        //                 _val = std::mem::transmute::<u16, i16>(val_u);
        //             }

        //             voice_data_vec.push(_val);
        //         }

        //         audio_service.lock().unwrap().add_user_voice_chunk(
        //             username,
        //             voice_data_vec,
        //             event_sink,
        //         );
        //     }
        // }

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
        type Aes128Ecb = Ecb<Aes128, Pkcs7>;
        let cipher = Aes128Ecb::new_from_slices(&self.secret_key, Default::default()).unwrap();
        let mut encrypt_packet = cipher.encrypt_vec(&binary_packet);

        let packet_size: u16 = encrypt_packet.len() as u16;
        let mut packet_size = bincode::serialize(&packet_size).unwrap();

        packet_size.append(&mut encrypt_packet);

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
        match udp_socket.peek(buf) {
            Ok(n) => {
                return Ok(n);
            }
            Err(e) => return Err(e),
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
