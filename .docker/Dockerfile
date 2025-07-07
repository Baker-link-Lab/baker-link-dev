FROM rust:1.84.0-bullseye

RUN rustup target install thumbv6m-none-eabi
RUN cargo install flip-link --version 0.1.10
RUN rustup component add rustfmt