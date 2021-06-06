# Silent
![](screenshot.png?raw=true)
Must be used with the [Silent Server](https://github.com/Flone-dnb/silent-server-rs) application.
# Build
<h3> 1. Install dependencies </h3>
<h4> Linux (Ubuntu/Debian) </h4>
<pre>
sudo apt install cmake
sudo apt install libfontconfig1-dev
sudo apt install libasound2-dev
sudo apt install libsfml-dev
sudo apt install libcsfml-dev
</pre>
<h4> Other </h4>
Install rust-sfml from https://github.com/jeremyletang/rust-sfml/wiki
<h3> 2. Build </h3>
Use 'cargo build --release' (requires Rust nightly) to build the app and copy the 'res' folder next to the binary.
