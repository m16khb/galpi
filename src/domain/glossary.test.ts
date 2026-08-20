import { describe, expect, test } from "bun:test"

import { type GlossaryEntry, usableGlossary } from "./glossary"

function entry(overrides: Partial<GlossaryEntry> & { id: string }): GlossaryEntry {
  return {
    id: overrides.id,
    term: overrides.term ?? "갈피",
    description: overrides.description ?? null,
  }
}

describe("usableGlossary", () => {
  test("drops a termless row and trims the rest", () => {
    // Given an edited glossary with one blank term row
    const edited = [
      entry({ id: "a", term: "  갈피 ", description: "  회의 녹음·전사 앱 " }),
      entry({ id: "b", term: "   " }),
    ]

    // When
    const usable = usableGlossary(edited)

    // Then
    expect(usable).toEqual([{ id: "a", term: "갈피", description: "회의 녹음·전사 앱" }])
  })

  test("keeps a term without a description", () => {
    expect(usableGlossary([entry({ id: "a", description: "  " })])).toEqual([
      { id: "a", term: "갈피", description: null },
    ])
  })

  test("turns a blank description into no description", () => {
    expect(usableGlossary([entry({ id: "a", description: " \n " })])[0]?.description).toBeNull()
  })
})
