set dotenv-filename := "just.env"

default:
	@just --list

client:
	cargo run --bin ddns_client

server:
	cargo run --bin ddns_server

publish major minor patch:
    docker logout
    echo "$CR_PAT" | docker login ghcr.io -u "$CR_USERNAME" --password-stdin

    docker build ./ddns_client/ -f ./ddns_client/Dockerfile -t "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}.{{minor}}.{{patch}}"

    docker tag "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}.{{minor}}.{{patch}}" "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}.{{minor}}"
    docker tag "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}.{{minor}}.{{patch}}" "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}"
    docker tag "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}.{{minor}}.{{patch}}" "ghcr.io/hadesmonsta/ddns_project:client-latest"

    docker push "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}.{{minor}}.{{patch}}"
    docker push "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}.{{minor}}"
    docker push "ghcr.io/hadesmonsta/ddns_project:client-v{{major}}"
    docker push "ghcr.io/hadesmonsta/ddns_project:client-latest"

    docker logout
    ./release_server.sh {{major}} {{minor}} {{patch}}
