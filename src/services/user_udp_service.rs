// External.
use bytevec::ByteDecodable;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::FromPrimitive;

// Std.
use std::io::ErrorKind;
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Custom.
use crate::global_params::*;
use crate::InternalMessage;

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum ServerMessageUdp {
    UserPing = 0,
    PingCheck = 1,
}

#[derive(Debug)]
pub struct UserUdpService {
    is_udp_connected: bool,
    io_udp_mutex: Mutex<()>,
}

impl UserUdpService {
    pub fn new() -> Self {
        UserUdpService {
            is_udp_connected: false,
            io_udp_mutex: Mutex::new(()),
        }
    }
    pub fn connect(&mut self, udp_socket: &UdpSocket, username: &str) -> Result<(), String> {
        let mut ok_buf = vec![0u8; 2];
        ok_buf[1] = username.len() as u8;

        let mut username_buf = Vec::from(username.as_bytes());
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
    pub fn peek(&self, udp_socket: &UdpSocket, buf: &mut [u8]) -> Result<usize, String> {
        loop {
            match udp_socket.peek(buf) {
                Ok(n) => {
                    return Ok(n);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(INTERVAL_UDP_MESSAGE_MS));
                    continue;
                }
                Err(e) => {
                    return Err(format!(
                        "udp_socket.peek_from() failed, error: {}, at [{}, {}]",
                        e,
                        file!(),
                        line!()
                    ));
                }
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
