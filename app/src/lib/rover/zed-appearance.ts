export const appearanceKey = "controlhub.zed-map.appearance.v1"
export const defaultAppearance = { pointSize: .025, brightness: 1, opacity: 1, colorMode: "rgb", viewMode: "points" }

export function parseAppearance(raw: string | null) {
  const result = { ...defaultAppearance }
  try {
    const value = JSON.parse(raw ?? "null")
    if (!value || typeof value !== "object") return result
    for (const [key, min, max] of [["pointSize", .005, .15], ["brightness", .25, 3], ["opacity", .1, 1]] as const) {
      if (typeof value[key] === "number" && Number.isFinite(value[key])) result[key] = Math.max(min, Math.min(max, value[key]))
    }
    if (["rgb", "height", "solid"].includes(value.colorMode)) result.colorMode = value.colorMode
    if (["points", "mesh"].includes(value.viewMode)) result.viewMode = value.viewMode
  } catch { /* Corrupt or older preferences should not prevent opening the map. */ }
  return result
}
