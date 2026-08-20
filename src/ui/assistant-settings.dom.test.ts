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

  test("masks a persisted key while keeping it available for saving", () => {
    // Given / When
    view.setSettings({
      apiKey: "zai_real_secret",
      model: "glm-5.2",
      baseUrl: null,
      reasoningEffort: "max",
      background: "제품: 갈피",
    })

    // Then
    expect(key.value).not.toContain("zai_real_secret")
    expect(key.readOnly).toBeTrue()
    expect(background.value).toBe("제품: 갈피")
    expect(view.settings()).toEqual({
      apiKey: "zai_real_secret",
      model: "glm-5.2",
      baseUrl: null,
      reasoningEffort: "max",
      background: "제품: 갈피",
    })
  })

  test("reveals and hides the persisted key with the eye toggle", () => {
    // Given
    view.setSettings({
      apiKey: "zai_real_secret",
      model: null,
      baseUrl: null,
      reasoningEffort: "max",
      background: null,
    })

    // When
    view.toggleVisibility()

    // Then
    expect(key.value).toBe("zai_real_secret")

    // When
    view.toggleVisibility()

    // Then
    expect(key.value).not.toContain("zai_real_secret")
  })

  test("falls back to the default model when none was saved", () => {
    // Given / When
    view.setSettings({
      apiKey: null,
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
      apiKey: null,
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
      apiKey: null,
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
      apiKey: null,
      model: null,
      baseUrl: null,
      reasoningEffort: "max",
      background: null,
    })

    // When
    background.value = "   \n  "

    // Then
    expect(view.settings().background).toBeNull()
    expect(view.settings().apiKey).toBeNull()
    expect(key.readOnly).toBeFalse()
  })

  test("keeps a newly typed key, model, base URL, and background for the next save", () => {
    // Given
    view.setSettings({
      apiKey: null,
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
    expect(view.settings()).toEqual({
      apiKey: "zai_typed",
      model: "openai/gpt-5.6",
      baseUrl: "https://openrouter.ai/api/v1",
      reasoningEffort: "max",
      background: "팀리더: 하빈",
    })
  })
})
