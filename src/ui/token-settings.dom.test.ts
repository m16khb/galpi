import { beforeEach, describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import { appTemplate } from "./app-template"
import { TokenSettingsView } from "./token-settings"

function createView(): { view: TokenSettingsView; input: HTMLInputElement } {
  const window = new Window()
  const root = window.document.createElement("div") as unknown as HTMLElement
  root.innerHTML = appTemplate
  window.document.body.appendChild(root as unknown as never)
  const view = new TokenSettingsView(root)
  const input = root.querySelector("#settings-hf-token") as HTMLInputElement
  return { view, input }
}

describe("TokenSettingsView (real DOM)", () => {
  let view: TokenSettingsView
  let input: HTMLInputElement

  beforeEach(() => {
    ;({ view, input } = createView())
  })

  test("masks a persisted token in the DOM and never exposes its value", () => {
    // Given / When
    view.setToken("hf_real_secret_value")

    // Then
    expect(input.value).not.toContain("hf_real_secret_value")
    expect(input.readOnly).toBeTrue()
    expect(view.token()).toBe("hf_real_secret_value")
  })

  test("reveals and hides the persisted token with the eye toggle", () => {
    // Given
    view.setToken("hf_real_secret_value")

    // When
    view.toggleVisibility()

    // Then
    expect(input.value).toBe("hf_real_secret_value")

    // When
    view.toggleVisibility()

    // Then
    expect(input.value).not.toContain("hf_real_secret_value")
  })

  test("keeps an empty field editable so a new token can be entered", () => {
    // Given / When
    view.setToken(null)

    // Then
    expect(input.value).toBe("")
    expect(input.readOnly).toBeFalse()
    expect(view.token()).toBe("")
  })

  test("clearing removes the persisted token and configured state", () => {
    // Given
    view.setToken("hf_real_secret_value")

    // When
    view.clearToken()

    // Then
    expect(view.token()).toBe("")
    const state = input.closest("body")?.querySelector("#token-configured-state") as HTMLElement
    expect(state.textContent).toBe("저장된 토큰 없음")
  })

  test("moves focus into the dialog when it opens", () => {
    // Given: the user opened settings from the topbar gear button
    const body = input.closest("body") as HTMLElement
    const trigger = body.querySelector(".settings-button") as HTMLElement
    trigger.focus()

    // When: the dialog opens
    view.show()

    // Then: focus lands on the close button inside the modal — it stays
    // enabled while setBusy(true) disables the fields during load (BUG-001)
    const close = body.querySelector(".settings-close-button") as HTMLElement
    expect(body.ownerDocument.activeElement).toBe(close)
    expect(body.ownerDocument.activeElement?.closest("#settings-dialog")).not.toBeNull()
  })

  test("returns focus to the invoking trigger when the dialog closes", () => {
    // Given: the user opened settings from the topbar gear button
    const body = input.closest("body") as HTMLElement
    const trigger = body.querySelector(".settings-button") as HTMLElement
    trigger.focus()
    view.show()
    input.focus()

    // When: the dialog closes (button, Escape, and programmatic paths share close())
    view.close()

    // Then: the keyboard user lands back on the invoker, not <body>
    expect(body.ownerDocument.activeElement).toBe(trigger)
  })
})
