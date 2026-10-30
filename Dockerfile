FROM --platform=$TARGETPLATFORM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        libudev-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --locked --release -p driver

FROM --platform=$TARGETPLATFORM ubuntu:24.04 AS runtime

COPY --from=ghcr.io/astral-sh/uv:0.12.5 /uv /uvx /bin/

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        iproute2 \
        libudev1 \
        network-manager \
        python3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/driver /usr/local/bin/driver
COPY data/ /data/
COPY crates/zed/sidecar/ /opt/controlhub/sidecar/

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/driver"]
CMD ["--config", "/data/config.json", "--sidecar", "/opt/controlhub/sidecar/main.py"]
