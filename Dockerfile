FROM --platform=$TARGETPLATFORM rust:1.88-bookworm AS builder

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

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        iproute2 \
        libudev1 \
        network-manager \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/driver /usr/local/bin/driver

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/driver"]
