# Silent

<p align="center">
  <img src="screenshot.png?raw=true" alt="Screenshot"/>
</p>

Must be used with the [Silent Server](https://github.com/Flone-dnb/silent-server-rs) application.

# Localization

The application is translated into the following languages: English, Russian.

The application itself does not contain a lot of text to translate so it could be easily translated into other languages.

If you want to translate the application into some other not supported language follow these steps:

- Edit the "localization.ods" file located in the "res" folder using LibreOffice Calc: add a new locale name in the first row (locale name should contain only 2 ASCII characters).
- Add translations to all keys.
- Save this file and also save it as .CSV format to the "res/localization.csv" file with default export settings.
- Submit the pull request to this repo with your changes.

# Build

### 1. Install dependencies

#### Linux (Debian based)

```
sudo apt install cmake libopenal-dev libfontconfig1-dev libasound2-dev libsfml-dev libcsfml-dev
```

#### Linux (Arch based)

```
sudo pacman -S cmake csfml sfml openal
```

#### Windows

Download [DLLs and LIBs from SFML](https://www.sfml-dev.org/files/SFML-2.5.1-windows-vc15-64-bit.zip) and build [rust-sfml](https://github.com/jeremyletang/rust-sfml/wiki).

### 2. Build

Use `cargo build --release` (requires Rust nightly) to build the app and copy the `res` directory next to the binary (+ SFML DLLs if you are on Windows).

# License

Please note, that starting from version **2.0.0** this project is licensed under the MIT license.

All versions prior to version **2.0.0** were licensed under the ZLib license.
