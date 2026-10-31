#!/bin/sh
set -eu

# Export only TensorRT from the host library mount. Adding the entire host
# directory to LD_LIBRARY_PATH would also override container libc/OpenCV.
mkdir -p /opt/controlhub/tensorrt
for library in /opt/jetson-libs/libnvinfer*.so* /opt/jetson-libs/libnvonnxparser*.so*; do
    [ -f "$library" ] || continue
    ln -sf "$library" "/opt/controlhub/tensorrt/$(basename "$library")"
done
exec "$@"
