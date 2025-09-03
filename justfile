set dotenv-filename := "just.env"

default:
    @just --list

client:
    cargo run --bin ddns_client

server:
    cargo run --bin ddns_server

publish major minor patch:
    mprocs --config mp_build.yaml
    ./release_server.sh {{ major }} {{ minor }} {{ patch }}
