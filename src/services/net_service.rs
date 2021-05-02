use std::io;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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

    pub fn start(&self, config: ClientConfig) {
        self.tokio_runtime.spawn(NetService::service(config));
    }

    pub fn stop(self) {
        self.tokio_runtime.shutdown_timeout(Duration::from_secs(5));
    }

    async fn service(config: ClientConfig) {
        let mut stream =
            TcpStream::connect(format!("{}:{}", config.server_name, config.server_port))
                .await
                .unwrap();

        stream.write_all(b"Hello world!").await.unwrap();
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
    }
}
