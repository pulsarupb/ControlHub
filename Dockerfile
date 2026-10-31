FROM --platform=$TARGETPLATFORM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        libclang-dev \
        libudev-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --locked --release -p driver

FROM --platform=$TARGETPLATFORM ubuntu:24.04 AS runtime

ARG ZED_SDK_VERSION=5.4
COPY --from=ghcr.io/astral-sh/uv:0.12.5 /uv /uvx /bin/

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        iproute2 \
        kmod \
        libegl1 \
        libgles2 \
        libgomp1 \
        libturbojpeg \
        libudev1 \
        libusb-1.0-0 \
        libv4l-0 \
        network-manager \
        python3 \
        python3-numpy \
        python3-opencv \
    && rm -rf /var/lib/apt/lists/*

# Install only the bindings: NumPy/OpenCV must keep Ubuntu's matching native ABI.
# The Jetson's SDK and CUDA are mounted by Compose, not copied from another L4T.
RUN uv pip install --system --break-system-packages --no-deps \
    "https://download.stereolabs.com/zedsdk/${ZED_SDK_VERSION}/whl/linux_aarch64/pyzed-${ZED_SDK_VERSION}-cp312-cp312-linux_aarch64.whl"

ENV LD_LIBRARY_PATH=/usr/local/zed/lib:/usr/local/cuda/lib64:/usr/lib/aarch64-linux-gnu/nvidia:/opt/controlhub/tensorrt
ENV LOGNAME=root

COPY --from=builder /app/target/release/driver /usr/local/bin/driver
COPY data/ /data/
COPY crates/zed/sidecar/ /opt/controlhub/sidecar/
COPY crates/driver/sidecars/oak/ /opt/controlhub/oak-sidecar/
COPY --chmod=755 docker-entrypoint.sh /usr/local/bin/controlhub-entrypoint

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/controlhub-entrypoint", "/usr/local/bin/driver"]
CMD ["--config", "/data/config.json", "--sidecar", "/opt/controlhub/sidecar/main.py", "--oak-sidecar", "/opt/controlhub/oak-sidecar/main.py"]
