#!/usr/bin/env bash
set -euxo pipefail

major="$1"
minor="$2"
patch="$3"
full="${major}.${minor}.${patch}"

docker logout
echo "$CR_PAT" | docker login ghcr.io -u "$CR_USERNAME" --password-stdin

docker build ./ -f docker/default/release/Dockerfile -t "ghcr.io/hadesmonsta/ddns_project:server-v${full}"
docker build ./ -f docker/nc/release/Dockerfile -t "ghcr.io/hadesmonsta/ddns_project:server-netcup-v${full}"

for tag_base in "server" "server-netcup"; do
    docker tag "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${full}" "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}"
    docker tag "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${full}" "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}.${minor}"
    docker tag "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${full}" "ghcr.io/hadesmonsta/ddns_project:${tag_base}-latest"

    docker push "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${full}"
    docker push "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}"
    docker push "ghcr.io/hadesmonsta/ddns_project:${tag_base}-v${major}.${minor}"
    docker push "ghcr.io/hadesmonsta/ddns_project:${tag_base}-latest"
done

docker logout
