import { expect, test } from "bun:test"

test("keeps hidden components out of layout when component styles set display", async () => {
  // Given
  const stylesheet = await Bun.file(new URL("./styles.css", import.meta.url)).text()

  // When
  const hiddenRule = stylesheet.match(/html\s+\[hidden\]\s*\{([^}]*)\}/u)

  // Then
  expect(hiddenRule?.at(1)).toMatch(/display:\s*none\s*;/u)
})
