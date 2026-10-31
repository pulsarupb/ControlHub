/// <reference types="bun" />
import { describe, expect, test } from "bun:test"
import { CAMPUS, insideMap, localPoint, validCoordinates, type OfflineMap } from "./rtk-map"

describe("offline RTK map", () => {
  test("campus projects to the map origin, north up and east right", () => {
    expect(localPoint(...CAMPUS)).toEqual([0, 0])
    expect(localPoint(CAMPUS[0] + .001, CAMPUS[1])[1]).toBeLessThan(0)
    expect(localPoint(CAMPUS[0], CAMPUS[1] + .001)[0]).toBeGreaterThan(0)
  })
  test("rejects missing, non-finite and out-of-projection coordinates", () => {
    for (const [lat, lon] of [[null, 26], [44, null], [NaN, 26], [44, Infinity], [90, 26], [44, 181]]) {
      expect(validCoordinates(lat, lon)).toBe(false)
    }
    expect(validCoordinates(0, 0)).toBe(true)
    expect(validCoordinates(...CAMPUS)).toBe(true)
  })
  test("bundled map covers campus, has valid geometry and detects outside fixes", async () => {
    const map = await Bun.file(new URL("../../../static/maps/politehnica.json", import.meta.url)).json() as OfflineMap
    expect(map.center).toEqual(CAMPUS)
    expect(insideMap(...CAMPUS, map.bounds)).toBe(true)
    expect(insideMap(45, 27, map.bounds)).toBe(false)
    expect(map.features.length).toBeGreaterThan(1000)
    expect(map.features.some(f => f.kind === "road" && f.name.length > 0)).toBe(true)
    expect(map.features.every(f => f.points.length >= 2 && f.points.every(p => p.length === 2 && p.every(Number.isFinite)))).toBe(true)
  })
})
