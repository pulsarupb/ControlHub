sudo ip link set can0 down
sudo ip link set can0 type can bitrate 1000000 dbitrate 5000000 fd on sample-point 0.666 dsample-point 0.666 restart-ms 100 berr-reporting on
sudo ip link set can0 up
ip -details -statistics link show can0
