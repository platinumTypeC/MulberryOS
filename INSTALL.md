# Installing

First Install rust:

## Linux
in bash `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.

## Windows
install rust from <a href="https://win.rustup.rs/x86_64"> here</a>.

## Switch to nightly
then run `rustup default nightly` to switch to nightly. Then run 
`rustup toolchain install nightly` to switch toolchain. then run `rustup component add --toolchain nightly rust-src` to install rust-src
