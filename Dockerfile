# syntax=docker/dockerfile:1
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

# Keep runtime assets and frontend changes out of the Rust build cache key.
COPY Cargo.toml Cargo.lock ./
COPY crates/d435/Cargo.toml crates/d435/
COPY crates/d435/src/ crates/d435/src/
COPY crates/driver/Cargo.toml crates/driver/
COPY crates/driver/src/ crates/driver/src/
COPY crates/kinova-control/Cargo.toml crates/kinova-control/build.rs crates/kinova-control/
COPY crates/kinova-control/src/ crates/kinova-control/src/
COPY crates/kinova-control/proto/ crates/kinova-control/proto/
COPY crates/oakd/Cargo.toml crates/oakd/
COPY crates/oakd/src/ crates/oakd/src/
COPY crates/roboteq/Cargo.toml crates/roboteq/
COPY crates/roboteq/src/ crates/roboteq/src/
COPY crates/rtk/Cargo.toml crates/rtk/
COPY crates/rtk/src/ crates/rtk/src/
COPY crates/zed/Cargo.toml crates/zed/
COPY crates/zed/src/ crates/zed/src/
# Embedded by include_str! in the ZED crate.
COPY crates/zed/sidecar/defaults.json crates/zed/sidecar/defaults.json

# Cargo loads every workspace member even when only building driver.
COPY app/src-tauri/Cargo.toml app/src-tauri/build.rs app/src-tauri/
COPY app/src-tauri/src/ app/src-tauri/src/
COPY app/src-tauri/native/Cargo.toml app/src-tauri/native/build.rs app/src-tauri/native/
COPY app/src-tauri/native/src/ app/src-tauri/native/src/
COPY app/src-tauri/native/switch-input/Cargo.toml app/src-tauri/native/switch-input/build.rs app/src-tauri/native/switch-input/
COPY app/src-tauri/native/switch-input/src/ app/src-tauri/native/switch-input/src/
COPY app/src-tauri/native/android-updater/Cargo.toml app/src-tauri/native/android-updater/build.rs app/src-tauri/native/android-updater/
COPY app/src-tauri/native/android-updater/src/ app/src-tauri/native/android-updater/src/

ARG TARGETARCH
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=controlhub-target-${TARGETARCH},target=/app/target,sharing=locked \
    cargo build --locked --release -p driver \
    && mkdir -p /out \
    && cp target/release/driver /out/driver

FROM --platform=$TARGETPLATFORM ubuntu:24.04 AS runtime

ARG ZED_SDK_VERSION=5.4
COPY --from=ghcr.io/astral-sh/uv:0.12.5 /uv /uvx /bin/

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        dbus-bin \
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

COPY --from=builder /out/driver /usr/local/bin/driver
COPY data/ /data/
COPY crates/zed/sidecar/ /opt/controlhub/sidecar/
COPY crates/oakd/sidecar/ /opt/controlhub/oak-sidecar/
COPY --chmod=755 docker-entrypoint.sh /usr/local/bin/controlhub-entrypoint

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/controlhub-entrypoint", "/usr/local/bin/driver"]
CMD ["--config", "/data/config.json", "--sidecar", "/opt/controlhub/sidecar/main.py", "--oak-sidecar", "/opt/controlhub/oak-sidecar/main.py"]
