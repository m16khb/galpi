import type { AssistantSettings } from "../domain/job"
import { nextTokenVisibility, tokenDisplayValue } from "./token-settings"

const KEY_SELECTOR = "#settings-assistant-key"
const MODEL_SELECTOR = "#settings-assistant-model"
const BASE_URL_SELECTOR = "#settings-assistant-base-url"
const EFFORT_SELECTOR = "#settings-assistant-effort"
const BACKGROUND_SELECTOR = "#settings-assistant-background"

export const DEFAULT_ASSISTANT_MODEL = "glm-5.3"

/** The assistant sheet owns credentials and context; the roster and glossary are edited beside it. */
export type AssistantCredentials = Omit<AssistantSettings, "participants" | "glossary">

export class AssistantSettingsView {
  private readonly root: HTMLElement
  private persistedKey: string | null = null
  private visible = false

  constructor(root: HTMLElement) {
    this.root = root
  }

  settings(): AssistantCredentials {
    const background = this.element<HTMLTextAreaElement>(BACKGROUND_SELECTOR).value.trim()
    const apiKey = this.persistedKey ?? this.element<HTMLInputElement>(KEY_SELECTOR).value.trim()
    const baseUrl = this.element<HTMLInputElement>(BASE_URL_SELECTOR).value.trim()
    const model = this.element<HTMLInputElement>(MODEL_SELECTOR).value.trim()
    return {
      apiKey: apiKey.length > 0 ? apiKey : null,
      model: model.length > 0 ? model : DEFAULT_ASSISTANT_MODEL,
      baseUrl: baseUrl.length > 0 ? baseUrl : null,
      reasoningEffort: this.element<HTMLSelectElement>(EFFORT_SELECTOR).value || null,
      background: background.length > 0 ? background : null,
    }
  }

  setSettings(settings: AssistantCredentials): void {
    const model = settings.model ?? DEFAULT_ASSISTANT_MODEL
    this.element<HTMLInputElement>(MODEL_SELECTOR).value = model
    this.element<HTMLInputElement>(BASE_URL_SELECTOR).value = settings.baseUrl ?? ""
    this.element<HTMLSelectElement>(EFFORT_SELECTOR).value =
      settings.reasoningEffort ?? (model.toLowerCase().startsWith("glm") ? "max" : "")
    this.element<HTMLTextAreaElement>(BACKGROUND_SELECTOR).value = settings.background ?? ""
    this.setPersistedKey(settings.apiKey)
  }

  setPersistedKey(apiKey: string | null): void {
    const input = this.element<HTMLInputElement>(KEY_SELECTOR)
    this.persistedKey = apiKey
    this.visible = false
    input.value = apiKey === null ? "" : tokenDisplayValue(apiKey, false)
    input.readOnly = apiKey !== null
    input.dataset["visible"] = "false"
    this.renderVisibility(false)
    this.setConfigured(apiKey !== null)
  }

  toggleVisibility(): void {
    const input = this.element<HTMLInputElement>(KEY_SELECTOR)
    this.visible = nextTokenVisibility(this.visible)
    if (this.persistedKey !== null) {
      input.value = tokenDisplayValue(this.persistedKey, this.visible)
    }
    input.dataset["visible"] = String(this.visible)
    this.renderVisibility(this.visible)
  }

  setConfigured(configured: boolean): void {
    const state = this.element("#assistant-configured-state")
    state.textContent = configured ? "토큰 저장됨" : "토큰 없음"
    state.dataset["state"] = configured ? "ready" : "pending"
  }

  setBusy(busy: boolean): void {
    this.element<HTMLInputElement>(KEY_SELECTOR).disabled = busy
    this.element<HTMLInputElement>(MODEL_SELECTOR).disabled = busy
    this.element<HTMLInputElement>(BASE_URL_SELECTOR).disabled = busy
    this.element<HTMLSelectElement>(EFFORT_SELECTOR).disabled = busy
    this.element<HTMLTextAreaElement>(BACKGROUND_SELECTOR).disabled = busy
    this.element<HTMLButtonElement>('[data-action="toggle-assistant-visibility"]').disabled = busy
  }

  private renderVisibility(visible: boolean): void {
    const button = this.element<HTMLButtonElement>("#toggle-assistant-visibility")
    button.setAttribute("aria-label", visible ? "토큰 숨기기" : "토큰 표시")
    button
      .querySelector<HTMLElement>("i")
      ?.setAttribute("class", visible ? "ph ph-eye-slash" : "ph ph-eye")
  }

  private element<T extends HTMLElement = HTMLElement>(selector: string): T {
    const element = this.root.querySelector<T>(selector)
    if (element === null) throw new Error(`필수 화면 요소가 없습니다: ${selector}`)
    return element
  }
}
