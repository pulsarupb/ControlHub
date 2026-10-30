import { rover } from "$lib/rover-control.svelte"
import { rover as display } from "$lib/data/rover.svelte"
import { hardware, releaseArm, armAction } from "$lib/hardware.svelte"
import { gamepadDemand } from "$lib/gamepad-mapping"

let frame: number | null = null
let lastToggle = false, lastStop = false, lastGripper: number | null = null
let controlling: "drive" | "arm" | null = null
let requireRelease = true

export function releaseGamepad() {
  if (controlling === "drive") rover.releaseJoystick()
  if (controlling === "arm") releaseArm()
  controlling = null
  requireRelease = true
}

export function setGamepadMode(mode: "drive" | "arm") {
  releaseGamepad()
  hardware.mode = mode
}

function poll() {
  const pad = Array.from(navigator.getGamepads?.() ?? []).find(p => p?.connected)
  display.gamepadConnected = Boolean(pad)
  display.gamepadId = pad?.id ?? "No gamepad connected"
  display.gamepadAxes = Array.from(pad?.axes ?? [0, 0, 0, 0])
  display.gamepadButtons = Array.from(pad?.buttons ?? [], b => b.pressed)
  const config = hardware.config?.gamepad
  if (!pad || !config || document.hidden || !document.hasFocus()) {
    releaseGamepad()
    lastToggle = lastStop = false
    lastGripper = null
  } else {
    const input = gamepadDemand(pad.axes, display.gamepadButtons, config)
    if (input.stop && !lastStop) { releaseGamepad(); void rover.stopRover() }
    if (input.toggle && !lastToggle) setGamepadMode(hardware.mode === "drive" ? "arm" : "drive")
    lastStop = input.stop; lastToggle = input.toggle
    const targetArmed = hardware.mode === "drive" ? !rover.stopped : Boolean(rover.status?.devices.kinova.data.armed)
    if (!targetArmed) releaseGamepad()
    // After a mode switch, disconnect, blur, or Stop, the operator must release
    // and press the deadman again. Holding it cannot carry motion across modes.
    if (!input.deadman) { releaseGamepad(); requireRelease = false }
    else if (!requireRelease && !input.stop && targetArmed) {
      if (hardware.mode === "drive" && !rover.mouseJoystickActive) {
        if (rover.startJoystick()) {
          controlling = "drive"; rover.setThrottle(input.throttle); rover.setSteering(input.steering)
        }
      } else if (hardware.mode === "arm" && rover.status?.devices.kinova.data.armed) {
        controlling = "arm"
        if (input.cartesian.some(v => v !== 0)) hardware.jog = { cartesian: input.cartesian }
        else releaseArm()
        if (input.gripper !== null && input.gripper !== lastGripper) void armAction({ action: "gripper", position: input.gripper })
      }
    }
    lastGripper = input.deadman ? input.gripper : null
  }
  frame = requestAnimationFrame(poll)
}

export function startGamepad() { if (frame === null) { requireRelease = true; poll() } }
export function stopGamepad() { if (frame !== null) cancelAnimationFrame(frame); frame = null; releaseGamepad() }
