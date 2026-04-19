FROM docker.io/rust:1-slim-bullseye AS build

RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY assets/ assets/
COPY dev/remote-pr-opener/Cargo.toml Cargo.toml
COPY dev/remote-pr-opener/Rocket.toml Rocket.toml
COPY dev/remote-pr-opener/src/ src/

RUN --mount=type=cache,target=/build/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    set -eux; \
    cargo build --release; \
    objcopy --compress-debug-sections target/release/remote-pr-opener ./main

################################################################################

FROM docker.io/debian:bullseye-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl1.1 git openssh-client
RUN update-ca-certificates

RUN git config --global user.email "pr-opener@eldolfin.top"
RUN git config --global user.name "pr-opener"

WORKDIR /app

COPY --from=build /build/main ./
COPY --from=build /build/Rocket.toml ./static

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=3000

CMD ["./main"]
