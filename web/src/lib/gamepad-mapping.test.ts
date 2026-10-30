/// <reference types="bun" />
import { describe, expect, test } from "bun:test"
import { gamepadDemand } from "./gamepad-mapping"
import type { GamepadConfig } from "./config-api"

const config: GamepadConfig = {
  dead_zone: .12, toggle_button: 3, deadman_button: 4, stop_button: 1,
  throttle_axis: 1, steering_axis: 2, arm_x_axis: 1, arm_y_axis: 0,
  arm_z_axis: 3, arm_yaw_axis: 2, pitch_up_button: 12, pitch_down_button: 13,
  roll_left_button: 14, roll_right_button: 15, gripper_open_button: 6, gripper_close_button: 7,
}
describe("shared gamepad defaults", () => {
  test("neutral and malformed axes cannot generate motion", () => {
    const input = gamepadDemand([NaN, .05, 0, 0], [], config)
    expect(input.throttle === 0).toBe(true); expect(input.cartesian.every(v => v === 0)).toBe(true)
    expect(input.deadman).toBe(false)
  })
  test("forward and arm XYZ use agreed axes and signs", () => {
    const input = gamepadDemand([1, -1, .56, -1], [], config)
    expect(input.throttle).toBe(1); expect(input.steering).toBeCloseTo(.5)
    expect(input.cartesian.slice(0, 3)).toEqual([1, -1, 1])
  })
  test("toggle, deadman, stop and gripper are independent", () => {
    const buttons = Array(17).fill(false); buttons[3] = buttons[4] = buttons[7] = true
    const input = gamepadDemand([], buttons, config)
    expect(input.toggle).toBe(true); expect(input.deadman).toBe(true)
    expect(input.stop).toBe(false); expect(input.gripper).toBe(1)
  })
})
