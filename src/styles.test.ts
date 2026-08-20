import { expect, test } from "bun:test"

test("keeps hidden components out of layout when component styles set display", async () => {
  // Given
  const stylesheet = await Bun.file(new URL("./styles.css", import.meta.url)).text()

  // When
  const hiddenRule = stylesheet.match(/html\s+\[hidden\]\s*\{([^}]*)\}/u)

  // Then
  expect(hiddenRule?.at(1)).toMatch(/display:\s*none\s*;/u)
})

test("workspace grid reserves a row for the app-error banner", async () => {
  // Given: the workspace shell stacks four children — topbar, #app-error
  // banner, scroll body, footer.
  const stylesheet = await Bun.file(new URL("./styles.css", import.meta.url)).text()

  // When
  const workspaceRule = stylesheet.match(/^\.workspace\s*\{([^}]*)\}/mu)
  const rows = workspaceRule?.at(1)?.match(/grid-template-rows:\s*([^;]+);/u)?.at(1)

  // Then: four row tracks — a three-track template collapses the banner's row
  // to 0px and the opaque body panel paints over the banner text (VQA-006).
  expect(rows?.trim()).toBe("auto auto minmax(0, 1fr) auto")
})
