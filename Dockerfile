FROM rust:latest as builder
COPY . .
RUN cargo build --release
CMD ["./target/release/team-event-picker"]
