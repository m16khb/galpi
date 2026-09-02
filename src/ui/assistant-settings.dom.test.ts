import { beforeEach, describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import { appTemplate } from "./app-template"
import { AssistantSettingsView, DEFAULT_ASSISTANT_MODEL } from "./assistant-settings"

function createView(): {
  view: AssistantSettingsView
  key: HTMLInputElement
  model: HTMLInputElement
  baseUrl: HTMLInputElement
  effort: HTMLSelectElement
  background: HTMLTextAreaElement
} {
  const window = new Window()
  const root = window.document.createElement("div") as unknown as HTMLElement
  root.innerHTML = appTemplate
  window.document.body.appendChild(root as unknown as never)
  return {
    view: new AssistantSettingsView(root),
    key: root.querySelector("#settings-assistant-key") as HTMLInputElement,
    model: root.querySelector("#settings-assistant-model") as HTMLInputElement,
    baseUrl: root.querySelector("#settings-assistant-base-url") as HTMLInputElement,
    effort: root.querySelector("#settings-assistant-effort") as HTMLSelectElement,
    background: root.querySelector("#settings-assistant-background") as HTMLTextAreaElement,
  }
}

describe("AssistantSettingsView (real DOM)", () => {
  let view: AssistantSettingsView
  let key: HTMLInputElement
  let model: HTMLInputElement
  let baseUrl: HTMLInputElement
  let effort: HTMLSelectElement
  let background: HTMLTextAreaElement

  beforeEach(() => {
    ;({ view, key, model, baseUrl, effort, background } = createView())
  })

  test("shows a stored key as a mask it never has to save again", () => {
    // Given / When: the host reports a key without sending its value
    view.setSettings({
      apiKeyStored: true,
      model: "glm-5.2",
      baseUrl: null,
      reasoningEffort: "max",
      background: "제품: 갈피",
    })

    // Then: the field is masked and read-only, and autosave carries nothing
    expect(key.value).not.toBe("")
    expect(key.readOnly).toBeTrue()
    expect(background.value).toBe("제품: 갈피")
    expect(view.pendingKey()).toBeNull()
    expect(view.settings()).toEqual({
      apiKeyStored: true,
      model: "glm-5.2",
      baseUrl: null,
      reasoningEffort: "max",
      background: "제품: 갈피",
    })
  })

  test("keeps reporting a stored key while unrelated fields are edited", () => {
    // Given: a stored key
    view.setSettings({
      apiKeyStored: true,
      model: null,
      baseUrl: null,
      reasoningEffort: "max",
      background: null,
    })

    // When: the user edits something else in the sheet
    background.value = "팀리더: 하빈"
    model.value = "glm-5.2"

    // Then: nothing about the key changed, so nothing about it is sent
    expect(view.pendingKey()).toBeNull()
    expect(view.settings().apiKeyStored).toBeTrue()
  })

  test("reveals and hides a key the user just typed with the eye toggle", () => {
    // Given: a key this window holds because it was entered here
    view.setKey("zai_real_secret")

    // When
    view.toggleVisibility()

    // Then
    expect(key.value).toBe("zai_real_secret")

    // When
    view.toggleVisibility()

    // Then
    expect(key.value).not.toContain("zai_real_secret")
  })

  test("hides the eye toggle for a key the window never received", () => {
    // Given / When
    view.setSettings({
      apiKeyStored: true,
      model: null,
      baseUrl: null,
      reasoningEffort: "max",
      background: null,
    })

    // Then: a value the window does not hold cannot be revealed
    const toggle = key.ownerDocument.querySelector("#toggle-assistant-visibility") as HTMLElement
    expect(toggle.hidden).toBeTrue()
  })

  test("clearing a stored key reopens the field for a new one", () => {
    // Given
    view.setSettings({
      apiKeyStored: true,
      model: null,
      baseUrl: null,
      reasoningEffort: "max",
      background: null,
    })

    // When
    view.clearKey()

    // Then
    expect(key.value).toBe("")
    expect(key.readOnly).toBeFalse()
    expect(view.settings().apiKeyStored).toBeFalse()
  })

  test("falls back to the default model when none was saved", () => {
    // Given / When
    view.setSettings({
      apiKeyStored: false,
      model: null,
      baseUrl: null,
      reasoningEffort: "max",
      background: null,
    })

    // Then
    expect(model.value).toBe(DEFAULT_ASSISTANT_MODEL)
    expect(view.settings().model).toBe(DEFAULT_ASSISTANT_MODEL)
  })

  test("keeps any provider model name in the free-form field", () => {
    // Given / When: an OpenRouter-style model id never present in suggestions
    view.setSettings({
      apiKeyStored: false,
      model: "anthropic/claude-sonnet-4",
      baseUrl: "https://openrouter.ai/api/v1",
      reasoningEffort: null,
      background: null,
    })

    // Then
    expect(model.value).toBe("anthropic/claude-sonnet-4")
    expect(view.settings().model).toBe("anthropic/claude-sonnet-4")
  })

  test("reports a blank base URL as absent so the default endpoint applies", () => {
    // Given
    view.setSettings({
      apiKeyStored: false,
      model: null,
      baseUrl: "https://openrouter.ai/api/v1",
      reasoningEffort: "max",
      background: null,
    })

    // When
    baseUrl.value = "   "

    // Then
    expect(view.settings().baseUrl).toBeNull()
  })

  test("reports blank background context as absent instead of empty text", () => {
    // Given
    view.setSettings({
      apiKeyStored: false,
      model: null,
      baseUrl: null,
      reasoningEffort: "max",
      background: null,
    })

    // When
    background.value = "   \n  "

    // Then
    expect(view.settings().background).toBeNull()
    expect(view.pendingKey()).toBeNull()
    expect(key.readOnly).toBeFalse()
  })

  test("keeps a newly typed key, model, base URL, and background for the next save", () => {
    // Given
    view.setSettings({
      apiKeyStored: false,
      model: null,
      baseUrl: null,
      reasoningEffort: "max",
      background: null,
    })

    // When
    key.value = "  zai_typed  "
    model.value = "openai/gpt-5.6"
    effort.value = "max"
    baseUrl.value = " https://openrouter.ai/api/v1 "
    background.value = "팀리더: 하빈"

    // Then
    expect(view.pendingKey()).toBe("zai_typed")
    expect(view.settings()).toEqual({
      apiKeyStored: true,
      model: "openai/gpt-5.6",
      baseUrl: "https://openrouter.ai/api/v1",
      reasoningEffort: "max",
      background: "팀리더: 하빈",
    })
  })
})
