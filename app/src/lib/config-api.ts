import { apiFetch } from "$lib/api-client"
import { roverConnection } from "$lib/rover-connection.svelte"

export type GamepadConfig = {
  dead_zone: number; toggle_button: number; deadman_button: number; stop_button: number;
  throttle_axis: number; steering_axis: number; arm_x_axis: number; arm_y_axis: number;
  arm_z_axis: number; arm_yaw_axis: number; pitch_up_button: number; pitch_down_button: number;
  roll_left_button: number; roll_right_button: number; gripper_open_button: number; gripper_close_button: number;
}
export type FullConfig = {
  drive: { enabled: boolean; ports: string[]; power_limit: number; acceleration_time_ms: number; deceleration_time_ms: number; motors: { controller: number; channel: number; direction: number }[];
    yaw_control?: { enabled: boolean; steering_yaw_sign: number; max_rate_rad_s: number; max_accel_rad_s2: number;
      kp: number; ki: number; correction_limit: number; filter_time_ms: number; feedback_timeout_ms: number;
      max_measured_rate_rad_s: number; heading_hold: boolean; heading_kp: number; steering_deadband: number } };
  kinova: { enabled: boolean; ip: string; port: number; username: string; password: string; linear_speed: number; angular_speed: number; joint_speed: number };
  rtk: { enabled: boolean; port: string; baud: number; ntrip: { enabled: boolean; host: string; port: number; mount_point: string; username: string; password: string } };
  zed: { enabled: boolean; python: string; settings: Record<string, Record<string, string | number | boolean>>; mount_configured: boolean; mount_translation: number[]; mount_rotation_deg: number[] };
  oak: { enabled: boolean; python: string; fps: number; cameras: { name: string; mxid: string; rotate_180: boolean }[] };
  gamepad: GamepadConfig;
}

export async function request<T>(
  path: string,
  body?: unknown,
  method = "POST",
  timeoutMs = 3000,
): Promise<T> {
  const response = await apiFetch(
    roverConnection.apiUrl(path),
    {
      method, headers: { "content-type": "application/json" },
      ...(body !== undefined ? { body: JSON.stringify(body) } : {}),
    },
    timeoutMs,
  )
  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: `HTTP ${response.status}` }))
    throw new Error(error.error ?? `HTTP ${response.status}`)
  }
  return response.json()
}
export const getConfig = () => request<FullConfig>("/api/config", undefined, "GET")
export const updateConfig = (config: FullConfig) => request("/api/config", config, "PUT")
