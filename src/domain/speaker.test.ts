import { describe, expect, test } from "bun:test"

import { buildSpeakerHint } from "./speaker"

describe("buildSpeakerHint", () => {
  test("builds an exact speaker hint", () => {
    const hint = buildSpeakerHint({
      mode: "exact",
      exact: 4,
      min: 2,
      max: 6,
    })

    expect(hint).toEqual({ mode: "exact", count: 4 })
  })

  test("rejects a reversed speaker range", () => {
    expect(() =>
      buildSpeakerHint({
        mode: "range",
        exact: 4,
        min: 7,
        max: 3,
      }),
    ).toThrow("최소 화자 수")
  })
})
