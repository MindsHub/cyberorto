#update
sudo apt update -y
#upgrade
sudo apt upgrade -y
#install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
#avr-hal dependencies:
sudo apt install avr-libc gcc-avr pkg-config avrdude libudev-dev build-essential
#vs-codium
#sudo apt install codium
#install ravedude
cargo +stable install ravedude
#check test
cargo test