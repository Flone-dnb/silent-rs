pub const CLIENT_CONFIG_FILE_NAME: &str = "silent.config";
pub const CONFIG_FILE_MAGIC_NUMBER: u16 = 51338;
pub const CONFIG_FILE_VERSION: u64 = 0;

pub const MAX_MESSAGE_SIZE: usize = 700;
pub const MAX_USERNAME_SIZE: usize = 25;
pub const MAX_MESSAGES_ON_SCREEN: usize = 100;
pub const DEFAULT_SERVER_PORT: u16 = 51337;

pub const INTERVAL_INTERNAL_MESSAGE_MS: u64 = 500;
pub const INTERVAL_TCP_MESSAGE_MS: u64 = 250;
pub const INTERVAL_TCP_CONNECT_MS: u64 = 50;
