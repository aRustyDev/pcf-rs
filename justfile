clean:
    rm -rf api/target
    rm -rf agent/target
    find ./data/ -type f | rg -v ".*(test)|(.gitkeep).*" | xargs -0 rm
    docker-compose down

run:
    docker-compose up

# Just an alias for build-all
build: build-all

build-all: build-api build-agent build-cli

build-api:
    just -f api/justfile build-docker

build-agent:
    just -f agent/justfile build

build-cli:
    just -f cli/justfile build
