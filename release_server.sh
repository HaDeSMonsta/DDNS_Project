#!/usr/bin/env bash
set -euxo pipefail

major="$1"
minor="$2"
patch="$3"

# Via mprocs in justfile
#docker build ./ -f docker/client/Dockerfile -t "ghcr.io/hadesmonsta/ddns_project:client-latest"
#docker build ./ -f docker/server/Dockerfile -t "ghcr.io/hadesmonsta/ddns_project:server-latest"
#docker build ./ -f docker/server/netcup/Dockerfile -t "ghcr.io/hadesmonsta/ddns_project:server-netcup-latest"

##############
### CLIENT ###
##############

docker tag "ghcr.io/hadesmonsta/ddns_project:client-latest" "ghcr.io/hadesmonsta/ddns_project:client-v${major}.${minor}.${patch}"
docker tag "ghcr.io/hadesmonsta/ddns_project:client-latest" "ghcr.io/hadesmonsta/ddns_project:client-v${major}.${minor}"
docker tag "ghcr.io/hadesmonsta/ddns_project:client-latest" "ghcr.io/hadesmonsta/ddns_project:client-v${major}"

docker push "ghcr.io/hadesmonsta/ddns_project:client-v${major}.${minor}.${patch}"
docker push "ghcr.io/hadesmonsta/ddns_project:client-v${major}.${minor}"
docker push "ghcr.io/hadesmonsta/ddns_project:client-v${major}"
docker push "ghcr.io/hadesmonsta/ddns_project:client-latest"

##############
### SERVER ###
##############

for tag_base in "server" "server-netcup"; do
    docker tag "ghcr.io/hadesmonsta/ddns_project:${tag_base}-latest" "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}.${minor}.${patch}"
    docker tag "ghcr.io/hadesmonsta/ddns_project:${tag_base}-latest" "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}.${minor}"
    docker tag "ghcr.io/hadesmonsta/ddns_project:${tag_base}-latest" "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}"

    docker push "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}.${minor}.${patch}"
    docker push "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}.${minor}"
    docker push "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}"
    docker push "ghcr.io/hadesmonsta/ddns_project:${tag_base}-latest"
done
