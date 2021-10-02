pub const CLIENT_CONFIG_FILE_NAME: &str = "silent.config";
pub const CONFIG_FILE_MAGIC_NUMBER: u16 = 51338;
pub const CONFIG_FILE_VERSION: u64 = 1;

pub const TEXT_SIZE: f64 = 18.0;
pub const MESSAGE_AUTHOR_TEXT_SIZE: f64 = 16.0;
pub const MESSAGE_TEXT_SIZE: f64 = 15.0;

// these should be in sync with the server global params
pub const MAX_MESSAGE_SIZE: usize = 500;
pub const MAX_USERNAME_SIZE: usize = 25;
pub const MAX_PASSWORD_SIZE: usize = 20;
pub const SPAM_PROTECTION_SEC: usize = 3; // (should be 'server value' + 1), can send only 1 message per SPAM_PROTECTION_SEC
pub const PASSWORD_RETRY_DELAY_SEC: usize = 6; // (should be 'server value' + 1)
pub const DEFAULT_ROOM_NAME: &str = "Lobby";

pub const MAX_MESSAGES_ON_SCREEN: usize = 50;
pub const DEFAULT_SERVER_PORT: u16 = 51337;

pub const INTERVAL_TCP_IDLE_MS: u64 = 250;
pub const INTERVAL_TCP_MESSAGE_MS: u64 = 10;
pub const INTERVAL_UDP_MESSAGE_MS: u64 = 2;

pub const IN_UDP_BUFFER_SIZE: usize = 1500;

pub const USER_CONNECT_FIRST_UDP_PING_RETRY_MAX_COUNT: u8 = 4; // when somebody connected and we already received his ping on UDP,
                                                               // but no info about user was received on TCP (so retry later)
pub const USER_CONNECT_FIRST_UDP_PING_RETRY_INTERVAL_MS: usize = 250; // try again after N ms

pub const MAX_WAIT_TIME_IN_VOICE_PLAYER_SEC: u64 = 3;

pub const NEW_MESSAGE_SOUND_PATH: &str = "res/sounds/newmessage.wav";
pub const CONNECTED_SOUND_PATH: &str = "res/sounds/connect.wav";
pub const DISCONNECT_SOUND_PATH: &str = "res/sounds/disconnect.wav";
pub const PUSH_TO_TALK_PRESS_SOUND: &str = "res/sounds/press.wav";
pub const PUSH_TO_TALK_UNPRESS_SOUND: &str = "res/sounds/unpress.wav";

pub const LOCALIZATION_FILE_PATH: &str = "res/localization.csv";
