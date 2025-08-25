default:
	@just --list

up:
	docker compose up -d --build

down:
	docker compose down

release_server_t tag="latest":
	clear
	docker login
	docker build -t "hadesmonsta/ddns_server:{{tag}}" -f ./ddns_server/docker/nc/release/Dockerfile ./ddns_server
	docker push "hadesmonsta/ddns_server:{{tag}}"
release_server tag:
	clear
	just release_server_t "{{tag}}"
	just release_server_t
