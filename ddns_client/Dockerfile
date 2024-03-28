FROM rust:latest as builder

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release

FROM debian:latest

COPY --from=builder /usr/src/app/target/release/ddns_client /usr/local/bin

CMD ["ddns_client"]
