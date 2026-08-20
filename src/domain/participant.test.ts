import { describe, expect, test } from "bun:test"

import {
  formatAliases,
  parseAliases,
  participantLabel,
  retainSelection,
  usableParticipants,
  type Participant,
} from "./participant"

function participant(overrides: Partial<Participant> & { id: string }): Participant {
  return { name: "하빈", role: null, aliases: [], ...overrides }
}

describe("parseAliases", () => {
  test("splits a comma separated field and drops blank entries", () => {
    expect(parseAliases(" 프로님 , 하빈님 ,, ")).toEqual(["프로님", "하빈님"])
  })

  test("returns nothing for an empty field", () => {
    expect(parseAliases("   ")).toEqual([])
  })

  test("round-trips through the edit field", () => {
    expect(parseAliases(formatAliases(["프로님", "하빈님"]))).toEqual(["프로님", "하빈님"])
  })
})

describe("participantLabel", () => {
  test("appends the role when one is saved", () => {
    expect(participantLabel(participant({ id: "a", role: "팀리더" }))).toBe("하빈 · 팀리더")
  })

  test("shows the bare name without a role", () => {
    expect(participantLabel(participant({ id: "a" }))).toBe("하빈")
  })
})

describe("usableParticipants", () => {
  test("drops a nameless row and trims the rest", () => {
    // Given a roster edited down to one blank row
    const edited = [
      participant({ id: "a", name: "  지우 ", role: "  백엔드 ", aliases: [" 지우님 ", "  "] }),
      participant({ id: "b", name: "   " }),
    ]

    // When
    const usable = usableParticipants(edited)

    // Then
    expect(usable).toEqual([{ id: "a", name: "지우", role: "백엔드", aliases: ["지우님"] }])
  })

  test("turns a blank role into no role", () => {
    expect(usableParticipants([participant({ id: "a", role: "  " })])[0]?.role).toBeNull()
  })
})

describe("retainSelection", () => {
  test("keeps only ids that still exist, in roster order", () => {
    // Given a selection made before someone was deleted from the roster
    const roster = [participant({ id: "a" }), participant({ id: "c" })]

    // When
    const kept = retainSelection(roster, new Set(["c", "b", "a"]))

    // Then
    expect(kept).toEqual(["a", "c"])
  })
})
