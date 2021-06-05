// External.
use bytevec::{ByteDecodable, ByteEncodable};
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::cast::ToPrimitive;
use num_traits::FromPrimitive;

// Std.
use std::io::ErrorKind;
use std::net::*;
use std::sync::MutexGuard;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use crate::global_params::*;
use crate::services::audio_service::audio_service::*;
use crate::InternalMessage;

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
    is_udp_connected: bool,
    io_udp_mutex: Mutex<()>,
    udp_socket_copy: Option<UdpSocket>,
    username: String,
}

impl UserUdpService {
    pub fn new() -> Self {
        UserUdpService {
            is_udp_connected: false,
            io_udp_mutex: Mutex::new(()),
            udp_socket_copy: None,
            username: String::from(""),
        }
    }
    pub fn assign_socket_and_name(&mut self, socket: UdpSocket, username: String) {
        self.udp_socket_copy = Some(socket);
        self.username = username;
    }
    pub fn send_voice_message(&mut self, voice_chunk: Vec<i16>) {
        // prepare voice packet:
        // (u8) - packet type (ClientMessageUdp::VoicePacket)
        // (u16) - voice data size in bytes
        // (size) - voice data

        let packet_id = ClientMessageUdp::VoicePacket.to_u8().unwrap();
        let voice_data_len: u16 = (voice_chunk.len() * std::mem::size_of::<i16>()) as u16;

        let voice_data_len_buf = u16::encode::<u16>(&voice_data_len);
        if let Err(e) = voice_data_len_buf {
            panic!(
                "An error occurred, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            );
        }
        let mut voice_data_len_buf = voice_data_len_buf.unwrap();

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

        let mut out_buf: Vec<u8> = Vec::new();
        out_buf.push(packet_id);
        out_buf.append(&mut voice_data_len_buf);
        out_buf.append(&mut voice_data);

        match self.send(self.udp_socket_copy.as_ref().unwrap(), &out_buf) {
            Err(msg) => {
                panic!("{}, at [{}, {}]", msg, file!(), line!());
            }
            _ => {}
        }
    }
    pub fn connect(&mut self, udp_socket: &UdpSocket) -> Result<(), String> {
        let mut ok_buf = vec![0u8; 2];
        ok_buf[1] = self.username.len() as u8;

        let mut username_buf = Vec::from(self.username.as_bytes());
        ok_buf.append(&mut username_buf);

        // Send:
        // (u8) - value '0',
        // (u8) - username.len(),
        // (size) - username
        if let Err(msg) = self.send(udp_socket, &ok_buf) {
            return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
        }

        // Receive '0' as OK and resend it again (it's first ping check).
        let mut ok_buf = vec![0u8; 1];
        if let Err(msg) = self.recv(&udp_socket, &mut ok_buf, 0) {
            return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
        }

        if ok_buf[0] != 0 {
            return Err(format!(
                "UserUdpService::connect() failed, error: received value is not '0', at [{}, {}]",
                file!(),
                line!()
            ));
        }

        // Resend.
        if let Err(msg) = self.send(udp_socket, &ok_buf) {
            return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn handle_message(
        &self,
        udp_socket: &UdpSocket,
        buf: &mut [u8],
        internal_messages: &Arc<Mutex<Vec<InternalMessage>>>,
        audio_service_guard: &mut MutexGuard<AudioService>,
    ) -> Result<(), String> {
        match FromPrimitive::from_u8(buf[0]) {
            Some(ServerMessageUdp::PingCheck) => {
                // It's ping update.
                // Resend this.
                match self.send(udp_socket, buf) {
                    Ok(()) => {}
                    Err(msg) => {
                        return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
                    }
                }
            }
            Some(ServerMessageUdp::UserPing) => {
                let username = Vec::from(&buf[2..2 + buf[1] as usize]);
                let username = String::from_utf8(username);
                if let Err(e) = username {
                    return Err(format!(
                        "String::from_utf8() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ));
                }
                let username = username.unwrap();

                let ping_buf =
                    &buf[2 + buf[1] as usize..2 + buf[1] as usize + std::mem::size_of::<u16>()];
                let ping_ms = u16::decode::<u16>(&ping_buf);
                if let Err(e) = ping_ms {
                    return Err(format!(
                        "u16::decode::<u16>() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ));
                }
                let ping_ms = ping_ms.unwrap();

                internal_messages
                    .lock()
                    .unwrap()
                    .push(InternalMessage::UserPing {
                        username,
                        ping_ms,
                        try_again_number: USER_CONNECT_FIRST_UDP_PING_RETRY_MAX_COUNT,
                    });
            }
            Some(ServerMessageUdp::VoiceMessage) => {
                // Packet structure:
                // (u8) - id (ServerMessageUdp::VoiceMessage)
                // (u8) - username len
                // (size) - username
                // (u16) - voice data len
                // (size) - voice data

                let username_len = buf[1];
                let mut _read_i = 2usize;
                let username = String::from_utf8(Vec::from(&buf[2..2 + username_len as usize]));
                _read_i += username_len as usize;
                if let Err(e) = username {
                    return Err(format!(
                        "String::from_utf8() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ));
                }
                let username = username.unwrap();

                // Read voice data len.
                let voice_data_len_buf = &buf[_read_i.._read_i + std::mem::size_of::<u16>()];
                _read_i += std::mem::size_of::<u16>();

                let voice_data_len = u16::decode::<u16>(&voice_data_len_buf);
                if let Err(e) = voice_data_len {
                    return Err(format!(
                        "u16::decode::<u16>() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ));
                }
                let voice_data_len = voice_data_len.unwrap();

                // Read voice data.
                let voice_data = &buf[_read_i.._read_i + voice_data_len as usize];
                let mut voice_data_vec: Vec<i16> = Vec::new();
                for i in (0..voice_data.len()).step_by(std::mem::size_of::<i16>()) {
                    let val_u = u16::decode::<u16>(&voice_data[i..i + std::mem::size_of::<i16>()]);
                    if let Err(e) = val_u {
                        return Err(format!(
                            "u16::decode::<u16>() failed, error: {}, at [{}, {}]",
                            e,
                            file!(),
                            line!()
                        ));
                    }
                    let val_u = val_u.unwrap();

                    let mut _val: i16 = 0;
                    unsafe {
                        _val = std::mem::transmute::<u16, i16>(val_u);
                    }

                    voice_data_vec.push(_val);
                }

                audio_service_guard.add_user_voice_chunk(
                    username,
                    voice_data_vec,
                    Arc::clone(&internal_messages),
                );
            }
            None => {
                return Err(format!(
                    "Unknown message received on UDP socket, message ID: {}, at [{}, {}]",
                    buf[0],
                    file!(),
                    line!()
                ));
            }
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
        loop {
            match udp_socket.peek(buf) {
                Ok(n) => {
                    return Ok(n);
                }
                Err(e) => return Err(e),
            }
        }
    }
    pub fn recv(
        &mut self,
        udp_socket: &UdpSocket,
        buf: &mut [u8],
        force_recv_size: usize,
    ) -> Result<(), String> {
        let _io_guard = self.io_udp_mutex.lock().unwrap();

        loop {
            match udp_socket.recv(buf) {
                Ok(n) => {
                    if force_recv_size != 0 {
                        if n != force_recv_size {
                            return Err(format!("udp_socket.recv() failed, error: received only {} bytes out of {}, at [{}, {}]",
                            n, force_recv_size, file!(), line!()));
                        } else {
                            break;
                        }
                    } else {
                        if n != buf.len() {
                            return Err(format!("udp_socket.recv() failed, error: received only {} bytes out of {}, at [{}, {}]",
                            n, buf.len(), file!(), line!()));
                        } else {
                            break;
                        }
                    }
                }
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

        Ok(())
    }
}
