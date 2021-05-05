use std::io;
use std::time::Duration;
use tokio::net::TcpStream;

use crate::services::user_net_service::*;

pub struct ClientConfig {
    pub username: String,
    pub server_name: String,
    pub server_port: String,
    pub server_password: String,
}

#[derive(Debug)]
pub struct NetService {
    tokio_runtime: tokio::runtime::Runtime,
}

impl NetService {
    pub fn new() -> Self {
        let rt = tokio::runtime::Runtime::new();
        if rt.is_err() {
            panic!("can't start Tokio runtime");
        }

        Self {
            tokio_runtime: rt.unwrap(),
        }
    }

    pub fn start(
        &self,
        config: ClientConfig,
        username: String,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
    ) {
        self.tokio_runtime
            .spawn(NetService::service(config, username, connect_layout_sender));
    }

    pub fn stop(self) {
        self.tokio_runtime.shutdown_timeout(Duration::from_secs(5));
    }

    async fn service(
        config: ClientConfig,
        username: String,
        connect_layout_sender: std::sync::mpsc::Sender<ConnectResult>,
    ) {
        let stream =
            TcpStream::connect(format!("{}:{}", config.server_name, config.server_port)).await;

        if stream.is_err() {
            connect_layout_sender.send(ConnectResult::OtherErr(
                String::from("Can't connect to the server. Make sure the specified server and port are correct, otherwise the server might be offline.")
            )).unwrap();
            return;
        }

        let mut stream = stream.unwrap();

        let mut user_net_service = UserNetService::new();
        match user_net_service.connect_user(&mut stream, username).await {
            ConnectResult::Ok => {
                connect_layout_sender.send(ConnectResult::Ok).unwrap();
            }
            res => {
                connect_layout_sender.send(res).unwrap();
                return;
            }
        }

        loop {
            // Wait for the socket to be readable
            stream.readable().await.unwrap();

            // Creating the buffer **after** the `await` prevents it from
            // being stored in the async task.
            let mut buf = [0; 4096];

            // Try to read data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            // (non-blocking)
            match stream.try_read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    println!("read {} bytes", n);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(_) => {
                    return println!("try_read() error");
                }
            }
        }

        println!("disconnected");
    }
}
