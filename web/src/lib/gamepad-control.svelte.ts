import { rover as controlRover } from "$lib/rover-control.svelte"
import { rover as displayRover } from "$lib/data/rover.svelte"

const AXIS_DEAD_ZONE = 0.12
const NUM_AXES = 4
const NUM_BUTTONS = 17

const Button = {
  CROSS: 0,
  CIRCLE: 1,
  SQUARE: 2,
  TRIANGLE: 3,
  L1: 4,
  R1: 5,
  L2: 6,
  R2: 7,
  SHARE: 8,
  OPTIONS: 9,
  L3: 10,
  R3: 11,
  DPAD_UP: 12,
  DPAD_DOWN: 13,
  DPAD_LEFT: 14,
  DPAD_RIGHT: 15,
  PS: 16,
} as const

let rafHandle: number | null = null
let wasInDeadZone = true
let prevButtons = new Array(NUM_BUTTONS).fill(false)
let prevConnected = false

function applyDeadZone(value: number): number {
  if (Math.abs(value) < AXIS_DEAD_ZONE) return 0
  return (value - Math.sign(value) * AXIS_DEAD_ZONE) / (1 - AXIS_DEAD_ZONE)
}

function poll() {
  const pads = navigator.getGamepads()
  const gamepad = Array.from(pads).find(Boolean)

  if (gamepad) {
    if (!prevConnected) {
      prevButtons.fill(false)
    }

    displayRover.gamepadConnected = true
    displayRover.gamepadId = gamepad.id
    displayRover.gamepadAxes = Array.from({ length: NUM_AXES }, (_, i) => gamepad.axes[i] ?? 0)
    displayRover.gamepadButtons = Array.from({ length: NUM_BUTTONS }, (_, i) => gamepad.buttons[i]?.pressed ?? false)

    const rawX = gamepad.axes[0] ?? 0
    const rawY = gamepad.axes[1] ?? 0
    const x = applyDeadZone(rawX)
    const y = applyDeadZone(rawY)
    const inDeadZone = Math.abs(x) < 0.01 && Math.abs(y) < 0.01

    if (inDeadZone) {
      if (!wasInDeadZone) {
        controlRover.releaseJoystick()
        wasInDeadZone = true
      }
    } else {
      if (wasInDeadZone) {
        controlRover.startJoystick()
        wasInDeadZone = false
      }
      controlRover.setJoystick(x, y)
    }

    const curr = gamepad.buttons

    if (curr[Button.CIRCLE]?.pressed && !prevButtons[Button.CIRCLE]) {
      controlRover.stopRover()
    }
    if (curr[Button.TRIANGLE]?.pressed && !prevButtons[Button.TRIANGLE]) {
      controlRover.resetRover()
    }
    if (curr[Button.SHARE]?.pressed && !prevButtons[Button.SHARE]) {
      controlRover.stopRover()
    }

    for (let i = 0; i < NUM_BUTTONS; i++) {
      prevButtons[i] = curr[i]?.pressed ?? false
    }

    prevConnected = true
  } else {
    if (prevConnected) {
      displayRover.gamepadConnected = false
      displayRover.gamepadId = "No gamepad connected"
      displayRover.gamepadAxes = [0, 0, 0, 0]
      displayRover.gamepadButtons = Array.from({ length: NUM_BUTTONS }, () => false)
      controlRover.releaseJoystick()
      prevConnected = false
    }
  }

  rafHandle = requestAnimationFrame(poll)
}

export function startGamepad() {
  if (rafHandle !== null) return
  wasInDeadZone = true
  prevConnected = false
  prevButtons.fill(false)
  poll()
}

export function stopGamepad() {
  if (rafHandle !== null) {
    cancelAnimationFrame(rafHandle)
    rafHandle = null
  }
  controlRover.releaseJoystick()
}
