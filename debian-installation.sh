#update
sudo apt update -y
#upgrade
sudo apt upgrade -y
#install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
#avr-hal dependencies:
sudo apt install avr-libc gcc-avr pkg-config avrdude libudev-dev build-essential -y
#vs-codium
#sudo apt install codium
#install ravedude
cargo +stable install ravedude
#check build arduino
cd arduino
cargo build --release
cd ..
#build and test orchestrator
cargo test