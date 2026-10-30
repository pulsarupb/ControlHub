import type { GamepadConfig } from "$lib/config-api"

export function gamepadDemand(axes: readonly number[], buttons: readonly boolean[], config: GamepadConfig) {
  const axis = (i: number) => {
    const v = axes[i] ?? 0
    if (!Number.isFinite(v) || Math.abs(v) <= config.dead_zone) return 0
    return Math.max(-1, Math.min(1, (v - Math.sign(v) * config.dead_zone) / (1 - config.dead_zone)))
  }
  const button = (i: number) => buttons[i] ? 1 : 0
  return {
    deadman: Boolean(button(config.deadman_button)),
    toggle: Boolean(button(config.toggle_button)),
    stop: Boolean(button(config.stop_button)),
    throttle: -axis(config.throttle_axis), steering: axis(config.steering_axis),
    cartesian: [-axis(config.arm_x_axis), -axis(config.arm_y_axis), -axis(config.arm_z_axis),
      button(config.roll_right_button) - button(config.roll_left_button),
      button(config.pitch_up_button) - button(config.pitch_down_button), axis(config.arm_yaw_axis)],
    gripper: button(config.gripper_close_button) ? 1 : button(config.gripper_open_button) ? 0 : null,
  }
}
