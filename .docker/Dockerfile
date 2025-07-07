FROM rust:1.84.0-bullseye

RUN rustup target install thumbv6m-none-eabi
RUN cargo install flip-link --version 0.1.10
RUN rustup component add rustfmt

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall -y elf2uf2-rs