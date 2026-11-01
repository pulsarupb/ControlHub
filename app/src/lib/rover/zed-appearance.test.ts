import { describe, expect, test } from "bun:test"
import { defaultAppearance, parseAppearance } from "./zed-appearance"

describe("ZED appearance preferences", () => {
  test("invalid storage falls back to defaults", () => {
    for (const value of [null, "", "broken", "null", "[]"]) expect(parseAppearance(value)).toEqual(defaultAppearance)
  })
  test("restores valid choices and clamps numeric settings", () => {
    expect(parseAppearance(JSON.stringify({ pointSize: 2, brightness: -1, opacity: .6, colorMode: "height", viewMode: "mesh" })))
      .toEqual({ pointSize: .15, brightness: .25, opacity: .6, colorMode: "height", viewMode: "mesh" })
  })
  test("rejects unexpected types and modes", () => {
    expect(parseAppearance('{"pointSize":".1","brightness":null,"opacity":false,"colorMode":"bad","viewMode":"bad"}'))
      .toEqual(defaultAppearance)
  })
})
