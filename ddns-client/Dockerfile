FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json --release
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
LABEL org.opencontainers.image.source=https://github.com/HaDeSMonsta/DDNS_Project
RUN apt update
RUN apt upgrade -y
RUN apt install curl -y
COPY --from=builder /app/target/release/ddns_client /usr/local/bin/ddns_client
CMD ["ddns_client"]
