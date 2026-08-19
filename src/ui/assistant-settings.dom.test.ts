import { beforeEach, describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import { appTemplate } from "./app-template"
import { AssistantSettingsView, DEFAULT_ASSISTANT_MODEL } from "./assistant-settings"

function createView(): {
  view: AssistantSettingsView
  key: HTMLInputElement
  model: HTMLSelectElement
  background: HTMLTextAreaElement
} {
  const window = new Window()
  const root = window.document.createElement("div") as unknown as HTMLElement
  root.innerHTML = appTemplate
  window.document.body.appendChild(root as unknown as never)
  return {
    view: new AssistantSettingsView(root),
    key: root.querySelector("#settings-assistant-key") as HTMLInputElement,
    model: root.querySelector("#settings-assistant-model") as HTMLSelectElement,
    background: root.querySelector("#settings-assistant-background") as HTMLTextAreaElement,
  }
}

describe("AssistantSettingsView (real DOM)", () => {
  let view: AssistantSettingsView
  let key: HTMLInputElement
  let model: HTMLSelectElement
  let background: HTMLTextAreaElement

  beforeEach(() => {
    ;({ view, key, model, background } = createView())
  })

  test("masks a persisted key while keeping it available for saving", () => {
    // Given / When
    view.setSettings({ apiKey: "zai_real_secret", model: "glm-5.2", background: "제품: 갈피" })

    // Then
    expect(key.value).not.toContain("zai_real_secret")
    expect(key.readOnly).toBeTrue()
    expect(background.value).toBe("제품: 갈피")
    expect(view.settings()).toEqual({
      apiKey: "zai_real_secret",
      model: "glm-5.2",
      background: "제품: 갈피",
    })
  })

  test("reveals and hides the persisted key with the eye toggle", () => {
    // Given
    view.setSettings({ apiKey: "zai_real_secret", model: null, background: null })

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
    view.setSettings({ apiKey: null, model: null, background: null })

    // Then
    expect(model.value).toBe(DEFAULT_ASSISTANT_MODEL)
    expect(view.settings().model).toBe(DEFAULT_ASSISTANT_MODEL)
  })

  test("keeps a saved model that is missing from the option list", () => {
    // Given / When
    view.setSettings({ apiKey: null, model: "glm-9-unreleased", background: null })

    // Then
    expect(model.value).toBe("glm-9-unreleased")
    expect(view.settings().model).toBe("glm-9-unreleased")
  })

  test("reports blank background context as absent instead of empty text", () => {
    // Given
    view.setSettings({ apiKey: null, model: null, background: null })

    // When
    background.value = "   \n  "

    // Then
    expect(view.settings().background).toBeNull()
    expect(view.settings().apiKey).toBeNull()
    expect(key.readOnly).toBeFalse()
  })

  test("keeps a newly typed key, model, and background for the next save", () => {
    // Given
    view.setSettings({ apiKey: null, model: null, background: null })

    // When
    key.value = "  zai_typed  "
    model.value = "glm-5-turbo"
    background.value = "팀리더: 하빈"

    // Then
    expect(view.settings()).toEqual({
      apiKey: "zai_typed",
      model: "glm-5-turbo",
      background: "팀리더: 하빈",
    })
  })
})
