import type { AssistantSettings } from "../domain/job"
import { nextTokenVisibility, tokenDisplayValue } from "./token-settings"

const KEY_SELECTOR = "#settings-assistant-key"
const MODEL_SELECTOR = "#settings-assistant-model"
const BACKGROUND_SELECTOR = "#settings-assistant-background"

export const DEFAULT_ASSISTANT_MODEL = "glm-5.3"

export class AssistantSettingsView {
  private readonly root: HTMLElement
  private persistedKey: string | null = null
  private visible = false

  constructor(root: HTMLElement) {
    this.root = root
  }

  settings(): AssistantSettings {
    const background = this.element<HTMLTextAreaElement>(BACKGROUND_SELECTOR).value.trim()
    const apiKey = this.persistedKey ?? this.element<HTMLInputElement>(KEY_SELECTOR).value.trim()
    return {
      apiKey: apiKey.length > 0 ? apiKey : null,
      model: this.element<HTMLSelectElement>(MODEL_SELECTOR).value,
      background: background.length > 0 ? background : null,
    }
  }

  setSettings(settings: AssistantSettings): void {
    const input = this.element<HTMLInputElement>(KEY_SELECTOR)
    this.persistedKey = settings.apiKey
    this.visible = false
    input.value = settings.apiKey === null ? "" : tokenDisplayValue(settings.apiKey, false)
    input.readOnly = settings.apiKey !== null
    input.dataset["visible"] = "false"
    this.renderVisibility(false)
    this.selectModel(settings.model ?? DEFAULT_ASSISTANT_MODEL)
    this.element<HTMLTextAreaElement>(BACKGROUND_SELECTOR).value = settings.background ?? ""
    this.setConfigured(settings.apiKey !== null)
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
    this.element<HTMLSelectElement>(MODEL_SELECTOR).disabled = busy
    this.element<HTMLTextAreaElement>(BACKGROUND_SELECTOR).disabled = busy
    this.element<HTMLButtonElement>('[data-action="toggle-assistant-visibility"]').disabled = busy
  }

  private selectModel(model: string): void {
    const select = this.element<HTMLSelectElement>(MODEL_SELECTOR)
    const known = [...select.options].some((option) => option.value === model)
    if (!known) {
      const option = select.ownerDocument.createElement("option")
      option.value = model
      option.textContent = `${model} · 저장된 설정`
      select.append(option)
    }
    select.value = model
  }

  private renderVisibility(visible: boolean): void {
    const button = this.element<HTMLButtonElement>("#toggle-assistant-visibility")
    button.setAttribute("aria-label", visible ? "토큰 숨기기" : "토큰 표시")
    button.querySelector<HTMLElement>("i")?.setAttribute(
      "class",
      visible ? "ph ph-eye-slash" : "ph ph-eye",
    )
  }

  private element<T extends HTMLElement = HTMLElement>(selector: string): T {
    const element = this.root.querySelector<T>(selector)
    if (element === null) throw new Error(`필수 화면 요소가 없습니다: ${selector}`)
    return element
  }
}
