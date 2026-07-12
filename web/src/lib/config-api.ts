import { apiFetch } from "$lib/api-client"
import { roverConnection } from "$lib/rover-connection.svelte"

export type MotorConfig = {
  id: number
  direction: number
}

export type MotorsConfig = {
  left_front: MotorConfig
  right_front: MotorConfig
  left_back: MotorConfig
  right_back: MotorConfig
}

export type ChassisConfigData = {
  wheel_radius_mm: number
  track_width_mm: number
  motor_rotations_per_wheel_rotation: number
  max_velocity: number
}

export type FullConfig = {
  chassis: ChassisConfigData
  motors: MotorsConfig
}

export async function getConfig(): Promise<FullConfig> {
  const response = await apiFetch(roverConnection.apiUrl("/api/config"))
  return response.json()
}

export async function updateConfig(config: FullConfig): Promise<void> {
  const response = await apiFetch(roverConnection.apiUrl("/api/config"), {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(config),
  })
  if (!response.ok) {
    const err = await response.json().catch(() => ({ error: "unknown error" }))
    throw new Error(err.error || `HTTP ${response.status}`)
  }
}
