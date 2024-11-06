FROM rust:latest AS builder

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:latest

COPY --from=builder /app/target/release/ddns_client /usr/local/bin

CMD ["ddns_client"]
