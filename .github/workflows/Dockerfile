FROM debian:13-slim

RUN apt-get update  \
    && apt-get -y --no-install-recommends install  \
        # install any other dependencies you might need
        sudo curl git ca-certificates build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
ENV MISE_DATA_DIR="/mise"
ENV MISE_CONFIG_DIR="/mise"
ENV MISE_CACHE_DIR="/mise/cache"
ENV MISE_INSTALL_PATH="/usr/local/bin/mise"
ENV PATH="/mise/shims:$PATH"

RUN curl https://mise.run | sh
COPY mise.toml /mise/config.toml
RUN mise install

WORKDIR /app

COPY tests/uv.lock tests/pyproject.toml tests/
RUN uv sync --project tests/
RUN uv run --project tests/ playwright install chromium --with-deps
