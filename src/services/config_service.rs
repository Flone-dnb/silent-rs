// External.
use bytevec::{ByteDecodable, ByteEncodable};
use platform_dirs::UserDirs;

// Std.
use std::io::prelude::*;
use std::path::Path;
use std::{fs::File, u16};

// Custom.
use crate::global_params::*;

#[derive(Debug)]
pub struct UserConfig {
    pub username: String,
    pub server: String,
    pub server_port: u16,
    pub server_password: String,
    pub ui_scaling: u16,
}

impl UserConfig {
    pub fn new() -> Result<UserConfig, String> {
        UserConfig::open()
    }
    pub fn save(&self) -> Result<(), String> {
        let config_path = UserConfig::get_config_file_path();
        if let Err(e) = config_path {
            return Err(format!("{} at [{}, {}]", e, file!(), line!()));
        }
        let mut config_path = config_path.unwrap();
        config_path += "~"; // save this first, then delete old one and rename this file

        if Path::new(&config_path).exists() {
            // Remove old temp file.
            if let Err(e) = std::fs::remove_file(&config_path) {
                return Err(format!("std::fs::remove_file() failed, error: can't remove temp config file (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()));
            }
        }

        // Create and write.
        let config_file = File::create(&config_path);
        if let Err(e) = config_file {
            return Err(format!(
                "File::create() failed, error: can't open config file '{}' (error: {}) at [{}, {}]",
                config_path,
                e,
                file!(),
                line!()
            ));
        }
        let mut config_file = config_file.unwrap();

        // Write magic number.
        let magic_number = CONFIG_FILE_MAGIC_NUMBER;
        let res = UserConfig::write_u16_to_file(&mut config_file, magic_number);
        if let Err(msg) = res {
            return Err(format!(
                "{} (writing magic number) at [{}, {}]",
                msg,
                file!(),
                line!()
            ));
        }

        // Write config version.
        let config_version = CONFIG_FILE_VERSION;
        let buf = u64::encode::<u64>(&config_version);
        if let Err(e) = buf {
            return Err(format!(
                "u64::encode::<u64>() failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        let buf = buf.unwrap();
        if let Err(e) = config_file.write(&buf) {
            return Err(format!(
                "File::write() failed, error: can't write config version to config file (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }

        // Write username len.
        let res = UserConfig::write_u16_to_file(&mut config_file, self.username.len() as u16);
        if let Err(msg) = res {
            return Err(format!(
                "{} (writing username len) at [{}, {}]",
                msg,
                file!(),
                line!()
            ));
        }

        // Write username.
        let res = UserConfig::write_string_to_file(&mut config_file, &self.username);
        if let Err(msg) = res {
            return Err(format!(
                "{} (writing username) at [{}, {}]",
                msg,
                file!(),
                line!()
            ));
        }

        // Write server len.
        let res = UserConfig::write_u16_to_file(&mut config_file, self.server.len() as u16);
        if let Err(msg) = res {
            return Err(format!(
                "{} (writing server len) at [{}, {}]",
                msg,
                file!(),
                line!()
            ));
        }

        // Write server.
        let res = UserConfig::write_string_to_file(&mut config_file, &self.server);
        if let Err(msg) = res {
            return Err(format!(
                "{} (writing server) at [{}, {}]",
                msg,
                file!(),
                line!()
            ));
        }

        // Write server port.
        let res = UserConfig::write_u16_to_file(&mut config_file, self.server_port);
        if let Err(msg) = res {
            return Err(format!(
                "{} (writing server port) at [{}, {}]",
                msg,
                file!(),
                line!()
            ));
        }

        // Write password len.
        let res =
            UserConfig::write_u16_to_file(&mut config_file, self.server_password.len() as u16);
        if let Err(msg) = res {
            return Err(format!(
                "{} (writing password len) at [{}, {}]",
                msg,
                file!(),
                line!()
            ));
        }

        if self.server_password.len() > 0 {
            // Write password.
            let res = UserConfig::write_string_to_file(&mut config_file, &self.server_password);
            if let Err(msg) = res {
                return Err(format!(
                    "{} (writing password) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
        }

        // Write ui scaling.
        let res = UserConfig::write_u16_to_file(&mut config_file, self.ui_scaling);
        if let Err(msg) = res {
            return Err(format!(
                "{} (writing ui scaling) at [{}, {}]",
                msg,
                file!(),
                line!()
            ));
        }

        // new settings go here...

        // Finished.
        drop(config_file);

        config_path.pop(); // pop '~'
        if Path::new(&config_path).exists() {
            // Remove old config file.
            if let Err(e) = std::fs::remove_file(&config_path) {
                return Err(format!("std::fs::remove_file() failed, error: can't remove old config file (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()));
            }
        }

        // Rename temp config file (with '~' to new config file).
        let old_name = String::from(&config_path) + "~";
        if let Err(e) = std::fs::rename(&old_name, &config_path) {
            return Err(format!(
                "std::fs::rename() failed, error: failed to rename temp file ({}) to ({}) (error: {}) at [{}, {}]",
                old_name,
                config_path,
                e,
                file!(),
                line!()
            ));
        }

        Ok(())
    }
    fn empty() -> UserConfig {
        UserConfig {
            username: String::from(""),
            server: String::from(""),
            server_port: DEFAULT_SERVER_PORT,
            server_password: String::from(""),
            ui_scaling: 100,
        }
    }
    fn open() -> Result<UserConfig, String> {
        let config_path = UserConfig::get_config_file_path();
        if let Err(e) = config_path {
            return Err(format!("{} at [{}, {}]", e, file!(), line!()));
        }
        let config_path = config_path.unwrap();

        if Path::new(&config_path).exists() {
            // Open and read existing file.
            let config_file = File::open(&config_path);
            if let Err(e) = config_file {
                return Err(format!(
                    "File::open() failed, error: can't open config file '{}' (error: {}) at [{}, {}]",
                    config_path,
                    e,
                    file!(),
                    line!()
                ));
            }
            let mut config_file = config_file.unwrap();

            let mut user_config = UserConfig::empty();

            // Read magic number.
            let magic_number = UserConfig::read_u16_from_file(&mut config_file);
            if let Err(msg) = magic_number {
                return Err(format!(
                    "{} (reading magic number) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            let magic_number = magic_number.unwrap();
            if magic_number != CONFIG_FILE_MAGIC_NUMBER {
                return Err(format!(
                    "An error occurred: file magic number ({}) != config magic number ({}) at [{}, {}]",
                    magic_number,
                    CONFIG_FILE_MAGIC_NUMBER,
                    file!(),
                    line!(),
                ));
            }

            // Read config version.
            let mut buf = vec![0u8; std::mem::size_of::<u64>()];
            if let Err(e) = config_file.read(&mut buf) {
                return Err(format!(
                    "File::read() failed, error: can't read config version from config file (error: {}) at [{}, {}]",
                    e,
                    file!(),
                    line!()
                ));
            }
            // use it to handle old config versions...
            let config_version = u64::decode::<u64>(&buf).unwrap();

            // Read username len.
            let username_len = UserConfig::read_u16_from_file(&mut config_file);
            if let Err(msg) = username_len {
                return Err(format!(
                    "{} (reading username len) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            let username_len = username_len.unwrap();

            // Read username.
            let username = UserConfig::read_string_from_file(&mut config_file, username_len);
            if let Err(msg) = username {
                return Err(format!(
                    "{} (reading username) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            user_config.username = username.unwrap();

            // Read server len.
            let server_len = UserConfig::read_u16_from_file(&mut config_file);
            if let Err(msg) = server_len {
                return Err(format!(
                    "{} (reading server len) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            let server_len = server_len.unwrap();

            // Read server.
            let server = UserConfig::read_string_from_file(&mut config_file, server_len);
            if let Err(msg) = server {
                return Err(format!(
                    "{} (reading server) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            user_config.server = server.unwrap();

            // Read server port.
            let server_port = UserConfig::read_u16_from_file(&mut config_file);
            if let Err(msg) = server_port {
                return Err(format!(
                    "{} (reading server port) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            user_config.server_port = server_port.unwrap();

            // Read password len.
            let password_len = UserConfig::read_u16_from_file(&mut config_file);
            if let Err(msg) = password_len {
                return Err(format!(
                    "{} (reading password len) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            let password_len = password_len.unwrap();

            if password_len > 0 {
                // Read password.
                let password = UserConfig::read_string_from_file(&mut config_file, password_len);
                if let Err(msg) = password {
                    return Err(format!(
                        "{} (reading password) at [{}, {}]",
                        msg,
                        file!(),
                        line!()
                    ));
                }
                user_config.server_password = password.unwrap();
            }

            // Read ui scaling.
            let ui_scaling = UserConfig::read_u16_from_file(&mut config_file);
            if let Err(msg) = ui_scaling {
                return Err(format!(
                    "{} (reading ui scaling) at [{}, {}]",
                    msg,
                    file!(),
                    line!()
                ));
            }
            user_config.ui_scaling = ui_scaling.unwrap();

            //
            // please use 'config_version' variable to handle old config versions...
            //

            Ok(user_config)
        } else {
            Ok(UserConfig::empty())
        }
    }
    fn get_config_file_path() -> Result<String, String> {
        let user_dirs = UserDirs::new();
        if user_dirs.is_none() {
            return Err(format!(
                "UserDirs::new() failed, error: can't read user dirs at [{}, {}]",
                file!(),
                line!(),
            ));
        }
        let user_dirs = user_dirs.unwrap();

        let config_dir = String::from(user_dirs.document_dir.to_str().unwrap());

        let mut _config_file_path = config_dir;
        if !_config_file_path.ends_with('/') && !_config_file_path.ends_with('\\') {
            _config_file_path += "/";
        }

        _config_file_path += CLIENT_CONFIG_FILE_NAME;

        Ok(_config_file_path)
    }
    fn read_u16_from_file(file: &mut File) -> Result<u16, String> {
        let mut buf = vec![0u8; std::mem::size_of::<u16>()];
        if let Err(e) = file.read(&mut buf) {
            return Err(format!(
                "File::read() failed, error: can't read u16 (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        Ok(u16::decode::<u16>(&buf).unwrap())
    }
    fn read_string_from_file(file: &mut File, string_len: u16) -> Result<String, String> {
        let mut buf = vec![0u8; string_len as usize];
        if let Err(e) = file.read(&mut buf) {
            return Err(format!(
                "File::read() failed, error: can't read string from config file (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        let string = String::from_utf8(buf);
        if let Err(e) = string {
            return Err(format!("String::from_utf8() failed, error: can't convert raw bytes (error: {}) at [{}, {}]",
            e,
            file!(),
            line!()));
        }
        Ok(string.unwrap())
    }
    fn write_u16_to_file(file: &mut File, val: u16) -> Result<(), String> {
        let buf = u16::encode::<u16>(&val);
        if let Err(e) = buf {
            return Err(format!(
                "u16::encode::<u16>() failed, error: {} at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        let buf = buf.unwrap();

        if let Err(e) = file.write(&buf) {
            return Err(format!(
                "File::write() failed, error: can't write u16 to config file (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        Ok(())
    }
    fn write_string_to_file(file: &mut File, string: &str) -> Result<(), String> {
        let buf = string.as_bytes();
        if let Err(e) = file.write(&buf) {
            return Err(format!(
                "File::write() failed, error: can't write string to config file (error: {}) at [{}, {}]",
                e,
                file!(),
                line!()
            ));
        }
        Ok(())
    }
}
