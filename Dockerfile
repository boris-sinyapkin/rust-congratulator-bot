FROM rust:latest AS builder

WORKDIR /myapp
COPY ./ .
RUN cargo build --release

# Run the image as a non-root user
RUN adduser -D myuser
USER myuser

ENV RUST_LOG=info
CMD ["./target/release/rust-congratulator-bot"]