import { friendlyError } from "$lib/friendly-error"
import { getConfig, request, type FullConfig } from "$lib/config-api"
import type { Preview } from "$lib/hardware-types"
import { rover } from "$lib/rover-control.svelte"

export const hardware = $state({
  config: null as FullConfig | null,
  mode: "drive" as "drive" | "arm",
  error: "",
  target: [0, 0, 0, 0, 0, 0],
  preview: null as Preview | null,
  jog: null as { cartesian: number[] } | { joints: number[] } | null,
})

export async function loadHardwareConfig() {
  try { hardware.config = await getConfig(); hardware.error = "" } catch (e) { hardware.error = friendlyError(e) }
}

export async function armAction(action: Record<string, unknown>, timeoutMs = 5000) {
  try {
    hardware.error = ""
    return await request<Preview>("/api/arm/action", action, "POST", timeoutMs)
  } catch (e) { hardware.error = friendlyError(e); return null }
}

export function releaseArm() {
  if (hardware.jog) {
    if (rover.driveTransport === "ws") {
      rover.releaseArmOverChannel()
    } else {
      void request("/api/arm/jog", { cartesian: [0, 0, 0, 0, 0, 0] }, "POST", 1000).catch(() => {})
    }
  }
  hardware.jog = null
}

let pending = false
export async function armTick() {
  if (rover.driveTransport === "ws") return
  if (pending || document.hidden || !document.hasFocus()) return
  pending = true
  try {
    await request("/api/heartbeat", undefined, "POST", 1000)
    if (hardware.jog) await request("/api/arm/jog", hardware.jog, "POST", 1000)
  } catch (e) { hardware.error = friendlyError(e); hardware.jog = null }
  finally { pending = false }
}

export async function solveTarget() {
  hardware.preview = null
  const target = [...hardware.target]
  const result = await armAction({ action: "solve", pose: target })
  if (JSON.stringify(target) === JSON.stringify(hardware.target)) hardware.preview = result
}
