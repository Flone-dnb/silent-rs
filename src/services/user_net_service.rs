use bytevec::{ByteDecodable, ByteEncodable};
use tokio::io::Interest;
use tokio::net::TcpStream;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

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

#[derive(Debug, PartialEq)]
pub enum ConnectResult {
    Ok,
    Err(IoResult),
    OtherErr(String),
}

#[derive(Debug, PartialEq)]
pub enum IoResult {
    Ok(usize),
    WouldBlock,
    FIN,
    Err(String),
}

pub struct UserNetService {
    pub user_state: UserState,
}

impl UserNetService {
    pub fn new() -> Self {
        UserNetService {
            user_state: UserState::NotConnected,
        }
    }
    pub async fn read_from_socket(&self, socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        // Wait for the socket to be readable
        if socket.readable().await.is_err() {
            return IoResult::Err(String::from("socket readable() failed"));
        }

        // (non-blocking)
        match socket.try_read(buf) {
            Ok(0) => {
                return IoResult::FIN;
            }
            Ok(n) => {
                if n != buf.len() {
                    return IoResult::Err(String::from(
                        "socket try_read() failed, error: failed to read 'buf_u16' size",
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
    pub async fn write_to_socket(&self, socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        // Wait for the socket to be writeable
        if socket.ready(Interest::WRITABLE).await.is_err() {
            return IoResult::Err(String::from("socket writeable() failed"));
        }

        // (non-blocking)
        match socket.try_write(buf) {
            Ok(0) => {
                return IoResult::FIN;
            }
            Ok(n) => {
                if n != buf.len() {
                    return IoResult::Err(String::from(
                        "socket try_write() failed, error: failed to read 'buf_u16' size",
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
    pub async fn connect_user(
        &mut self,
        socket: &mut TcpStream,
        username: String,
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
            match self.write_to_socket(socket, &mut out_buffer).await {
                IoResult::WouldBlock => continue, // try again
                IoResult::Ok(_bytes) => break,
                res => return ConnectResult::Err(res),
            };
        }

        // Wait for answer.
        let mut in_buf = vec![0u8; std::mem::size_of::<u16>()];
        loop {
            match self.read_from_socket(socket, &mut in_buf).await {
                IoResult::WouldBlock => continue, // try again
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
                    match self.read_from_socket(socket, &mut in_buf).await {
                        IoResult::WouldBlock => continue, // try again
                        IoResult::Ok(_bytes) => break,
                        res => return ConnectResult::Err(res),
                    }
                }
                let required_ver_str_size = u16::decode::<u16>(&in_buf).unwrap();

                // Get correct version string.
                let mut required_ver_str_buf = vec![0u8; required_ver_str_size as usize];
                loop {
                    match self.read_from_socket(socket, &mut required_ver_str_buf).await {
                        IoResult::WouldBlock => continue, // try again
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

        return ConnectResult::Ok;
    }
}
