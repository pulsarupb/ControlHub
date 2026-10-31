// Refresh the bundled offline map. Run from app/: bun scripts/build-rtk-map.ts
// Never downloads tiles: uses OSM vector data via Overpass, with attribution.
import { mkdir } from "node:fs/promises"

const center = [44.4385, 26.048] as const
const bounds = [44.3935, 25.985, 44.4835, 26.111] as const // south, west, north, east
const bbox = bounds.join(",")
const query = `[out:json][timeout:180];(way[highway](${bbox});way[building](${bbox});way[leisure~"park|garden|pitch"](${bbox});way[natural~"water|wood"](${bbox});way[landuse~"forest|grass|recreation_ground"](${bbox});way[waterway](${bbox});way[railway=rail](${bbox}););out geom;`
type Way = { tags: Record<string, string>; geometry?: { lat: number; lon: number }[] }
const radius = 6378137
const project = (lat: number, lon: number) => [radius * lon * Math.PI / 180, -radius * Math.log(Math.tan(Math.PI / 4 + lat * Math.PI / 360))]
const origin = project(...center)
let data: { elements: Way[] } | undefined
for (const endpoint of ["https://overpass-api.de/api/interpreter", "https://overpass.kumi.systems/api/interpreter"]) {
  try {
    console.log(`Fetching Politehnica map from ${endpoint}…`)
    const response = await fetch(endpoint, { method: "POST", body: new URLSearchParams({ data: query }), signal: AbortSignal.timeout(240000) })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    data = await response.json() as { elements: Way[] }
    break
  } catch (error) { console.warn(String(error)) }
}
if (!data?.elements?.length) throw new Error("No map data received; existing map was not changed")
const features = data.elements.flatMap(way => {
  if (!way.geometry || way.geometry.length < 2) return []
  const t = way.tags
  const kind = t.highway ? "road" : t.building ? "building" : t.natural === "water" ? "water" : t.waterway ? "waterway" : t.railway ? "rail" : "green"
  const points = way.geometry.map(p => {
    const xy = project(p.lat, p.lon)
    return xy.map((v, i) => Math.round((v - origin[i]) * 10) / 10)
  })
  return [{ kind, detail: t.highway ?? "", name: t.name ?? "", points }]
})
await mkdir("static/maps", { recursive: true })
await Bun.write("static/maps/politehnica.json", JSON.stringify({ center, bounds, generated: new Date().toISOString(), features }))
console.log(`Saved ${features.length} features to static/maps/politehnica.json`)
