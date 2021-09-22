# Silent
<p align="center">
  <img width="400" height="300" src="screenshot.png">
</p>
Must be used with the [Silent Server](https://github.com/Flone-dnb/silent-server-rs) application.
# Build
<h3> 1. Install dependencies </h3>
<h4> Linux (Debian based) </h4>
<pre>
sudo apt install cmake libopenal-dev libfontconfig1-dev libasound2-dev libsfml-dev libcsfml-dev
</pre>
<h4> Linux (Arch based) </h4>
<pre>
sudo pacman -S cmake csfml sfml openal
</pre>
<h4> Other </h4>
Install OpenAL (for Windows: https://www.openal.org/downloads/ (choose windows installer)) and rust-sfml from https://github.com/jeremyletang/rust-sfml/wiki
<h3> 2. Build </h3>
Use 'cargo build --release' (requires Rust nightly) to build the app and copy the 'res' folder next to the binary.
# License
Please note, that starting from version **2.0.0** this project is licensed under the MIT license.<br>
All versions prior to version **2.0.0** were licensed under the ZLib license.
