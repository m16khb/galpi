import { expect, test } from "bun:test"

import { appTemplate } from "./app-template"
import { nextTokenVisibility, tokenDisplayValue } from "./token-settings"

test("toggles persisted token visibility without changing the value", () => {
  expect(nextTokenVisibility(false)).toBeTrue()
  expect(nextTokenVisibility(true)).toBeFalse()
  expect(tokenDisplayValue("secret-token", false)).not.toContain("secret-token")
  expect(tokenDisplayValue("secret-token", true)).toBe("secret-token")
})

test("uses a non-credential input to avoid WebView password restoration", () => {
  const field = appTemplate.match(/<input id="settings-hf-token"[^>]*>/u)?.at(0)

  expect(field).toContain('autocomplete="off"')
  expect(field).toContain('type="text"')
})
