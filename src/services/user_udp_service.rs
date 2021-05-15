// External.
use bytevec::ByteDecodable;

// Std.
use std::io::ErrorKind;
use std::net::*;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// Custom.
use crate::global_params::*;

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
    pub fn connect(&mut self, udp_socket: &UdpSocket, username: &str) -> Result<u16, String> {
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
        if let Err(msg) = self.recv(&udp_socket, &mut ok_buf) {
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

        let mut ping_buf = vec![0u8; std::mem::size_of::<u16>()];
        let mut _ping_ms = 0;

        // Receive ping.
        if let Err(msg) = self.recv(&udp_socket, &mut ping_buf) {
            return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
        }

        // Read ping.
        let ping_ms = u16::decode::<u16>(&ping_buf);
        if let Err(e) = ping_ms {
            return Err(format!(
                "u16::decode::<u16>() failed, error: {}, at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        _ping_ms = ping_ms.unwrap();

        Ok(_ping_ms)
    }
    pub fn send(&mut self, udp_socket: &UdpSocket, buf: &[u8]) -> Result<(), String> {
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
    pub fn recv(&mut self, udp_socket: &UdpSocket, buf: &mut [u8]) -> Result<(), String> {
        let _io_guard = self.io_udp_mutex.lock().unwrap();

        loop {
            match udp_socket.recv(buf) {
                Ok(n) => {
                    if n != buf.len() {
                        return Err(format!("udp_socket.recv() failed, error: received only {} bytes out of {}, at [{}, {}]",
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
