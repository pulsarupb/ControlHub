export type Point = [number, number]
export type OfflineMap = {
  center: Point
  bounds: [number, number, number, number]
  generated: string
  features: { kind: string; detail: string; name: string; points: Point[] }[]
}
export const CAMPUS: Point = [44.4385, 26.048]
const RADIUS = 6378137
export function project(latitude: number, longitude: number): Point {
  return [RADIUS * longitude * Math.PI / 180, -RADIUS * Math.log(Math.tan(Math.PI / 4 + latitude * Math.PI / 360))]
}
const origin = project(...CAMPUS)
export function localPoint(latitude: number, longitude: number): Point {
  const p = project(latitude, longitude)
  return [p[0] - origin[0], p[1] - origin[1]]
}
export function validCoordinates(latitude: unknown, longitude: unknown): boolean {
  return typeof latitude === "number" && Number.isFinite(latitude) && Math.abs(latitude) <= 85.05112878
    && typeof longitude === "number" && Number.isFinite(longitude) && Math.abs(longitude) <= 180
}
export function insideMap(latitude: number, longitude: number, bounds: OfflineMap["bounds"]): boolean {
  return latitude >= bounds[0] && latitude <= bounds[2] && longitude >= bounds[1] && longitude <= bounds[3]
}
