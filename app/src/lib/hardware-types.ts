export type Device<T> = {
  enabled: boolean; connected: boolean; simulated: boolean;
  updated_at: number | null; error: string | null; data: T;
}
export type CameraPose = { translation: [number, number, number]; orientation: [number, number, number, number] }
export type ZedMap = {
  camera_ready: boolean; camera_model: string; serial: string; fps: number;
  tracking_state: string; mapping_state: string; pose: CameraPose | null;
  path: CameraPose[]; points: number[][]; colors: number[][];
  imu: { acceleration: number[]; angular_velocity: number[] } | null; error: string | null;
}
export type Preview = { id: number; target: number[]; joints: number[]; seed: number[]; expires_at: number }
export type RearBoundary = {
  forward: [number, number]; wall_offset_m: number; zone_length_m: number;
  zone_width_m: number; zone_height_m: number; controller_zone_installed: boolean;
}
export type Devices = {
  drive: Device<{ armed: boolean; power: number[][]; controllers: { encoder_counts: number[]; speed_rpm: number[]; faults: number }[] }>;
  kinova: Device<{ armed: boolean; joints: number[]; pose: number[] | null; gripper: number | null; arm_state: number | null; preview: Preview | null;
    rear_boundary: RearBoundary | null; rear_boundary_error: string | null; telemetry_error: string | null }>;
  rtk: Device<{ fix: { latitude: number | null; longitude: number | null; altitude_msl: number | null; quality: number; satellites: number; hdop: number | null; received_at: number; latitude_std: number } | null;
    bytes_read: number; parse_errors: number; ntrip_connected: boolean; correction_bytes: number; ntrip_error: string | null }>;
  zed: Device<ZedMap>;
}
