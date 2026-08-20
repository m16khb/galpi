import { describe, expect, test } from "bun:test"

import {
  formatAliases,
  type Participant,
  parseAliases,
  participantLabel,
  retainSelection,
  usableParticipants,
} from "./participant"

function participant(overrides: Partial<Participant> & { id: string }): Participant {
  return {
    id: overrides.id,
    name: overrides.name ?? "하빈",
    team: overrides.team ?? null,
    role: overrides.role ?? null,
    description: overrides.description ?? null,
    aliases: overrides.aliases ?? [],
  }
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

  test("appends team and role together when both are saved", () => {
    expect(participantLabel(participant({ id: "a", team: "갈피팀", role: "팀리더" }))).toBe(
      "하빈 · 갈피팀 · 팀리더",
    )
  })

  test("appends the team alone when no role is saved", () => {
    expect(participantLabel(participant({ id: "a", team: "갈피팀" }))).toBe("하빈 · 갈피팀")
  })

  test("shows the bare name without a role", () => {
    expect(participantLabel(participant({ id: "a" }))).toBe("하빈")
  })
})

describe("usableParticipants", () => {
  test("drops a nameless row and trims the rest", () => {
    // Given a roster edited down to one blank row
    const edited = [
      participant({
        id: "a",
        name: "  지우 ",
        team: " 갈피팀 ",
        role: "  백엔드 ",
        description: " 녹음 파이프라인 담당 ",
        aliases: [" 지우님 ", "  "],
      }),
      participant({ id: "b", name: "   " }),
    ]

    // When
    const usable = usableParticipants(edited)

    // Then
    expect(usable).toEqual([
      {
        id: "a",
        name: "지우",
        team: "갈피팀",
        role: "백엔드",
        description: "녹음 파이프라인 담당",
        aliases: ["지우님"],
      },
    ])
  })

  test("turns a blank role into no role", () => {
    expect(usableParticipants([participant({ id: "a", role: "  " })])[0]?.role).toBeNull()
  })

  test("turns a blank team into no team", () => {
    expect(usableParticipants([participant({ id: "a", team: "  " })])[0]?.team).toBeNull()
  })

  test("turns a blank description into no description", () => {
    expect(
      usableParticipants([participant({ id: "a", description: "  " })])[0]?.description,
    ).toBeNull()
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
