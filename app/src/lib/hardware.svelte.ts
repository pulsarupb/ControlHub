import { getConfig, request, type FullConfig } from "$lib/config-api"
import type { Preview } from "$lib/hardware-types"

export const hardware = $state({
  config: null as FullConfig | null,
  mode: "drive" as "drive" | "arm",
  error: "",
  target: [0, 0, 0, 0, 0, 0],
  preview: null as Preview | null,
  jog: null as { cartesian: number[] } | { joints: number[] } | null,
})

export async function loadHardwareConfig() {
  try { hardware.config = await getConfig(); hardware.error = "" } catch (e) { hardware.error = String(e) }
}

export async function armAction(action: Record<string, unknown>, timeoutMs = 5000) {
  try {
    hardware.error = ""
    return await request<Preview>("/api/arm/action", action, "POST", timeoutMs)
  } catch (e) { hardware.error = String(e); return null }
}

export function releaseArm() {
  if (hardware.jog) void request("/api/arm/jog", { cartesian: [0, 0, 0, 0, 0, 0] }, "POST", 1000).catch(() => {})
  hardware.jog = null
}

let pending = false
export async function armTick() {
  if (pending || document.hidden || !document.hasFocus()) return
  pending = true
  try {
    await request("/api/heartbeat", undefined, "POST", 1000)
    if (hardware.jog) await request("/api/arm/jog", hardware.jog, "POST", 1000)
  } catch (e) { hardware.error = String(e); hardware.jog = null }
  finally { pending = false }
}

export async function solveTarget() {
  hardware.preview = null
  const target = [...hardware.target]
  const result = await armAction({ action: "solve", pose: target })
  if (JSON.stringify(target) === JSON.stringify(hardware.target)) hardware.preview = result
}
