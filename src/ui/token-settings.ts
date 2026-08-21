const MASKED_TOKEN = "••••••••••••"

export function nextTokenVisibility(visible: boolean): boolean {
  return !visible
}

export function tokenDisplayValue(token: string, visible: boolean): string {
  return visible ? token : MASKED_TOKEN
}

export class TokenSettingsView {
  private readonly root: HTMLElement
  private persistedToken: string | null = null
  private visible = false
  private previouslyFocused: HTMLElement | null = null

  constructor(root: HTMLElement) {
    this.root = root
    this.root.ownerDocument.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && !this.element("#settings-dialog").hidden) this.close()
    })
  }

  show(): void {
    // APG dialog pattern: remember the invoker so close() can return the
    // keyboard user to where they were instead of dropping to <body>.
    this.previouslyFocused = this.root.ownerDocument.activeElement as HTMLElement | null
    this.element("#settings-dialog").hidden = false
    this.showMessage("")
    // APG dialog pattern: move focus into the dialog on open so keyboard and
    // screen-reader users land inside the modal, not on the invoker behind it.
    // The close button is the target because it stays enabled while setBusy()
    // disables the fields during load (BUG-001).
    this.element<HTMLButtonElement>(".settings-close-button").focus()
  }

  close(): void {
    this.element("#settings-dialog").hidden = true
    const restore = this.previouslyFocused
    this.previouslyFocused = null
    if (restore?.isConnected) restore.focus()
  }

  token(): string {
    return this.persistedToken ?? this.element<HTMLInputElement>("#settings-hf-token").value
  }

  setToken(token: string | null): void {
    const input = this.element<HTMLInputElement>("#settings-hf-token")
    this.persistedToken = token
    this.visible = false
    input.value = token === null ? "" : tokenDisplayValue(token, false)
    input.readOnly = token !== null
    input.dataset["visible"] = "false"
    this.renderVisibility(false)
    this.setConfigured(token !== null)
  }

  clearToken(): void {
    this.setToken(null)
  }

  toggleVisibility(): void {
    const input = this.element<HTMLInputElement>("#settings-hf-token")
    this.visible = nextTokenVisibility(this.visible)
    if (this.persistedToken !== null) {
      input.value = tokenDisplayValue(this.persistedToken, this.visible)
    }
    input.dataset["visible"] = String(this.visible)
    this.renderVisibility(this.visible)
  }

  setConfigured(configured: boolean): void {
    const state = this.element("#token-configured-state")
    state.textContent = configured ? "저장됨" : "저장된 토큰 없음"
    state.dataset["state"] = configured ? "ready" : "pending"
  }

  setBusy(busy: boolean): void {
    this.element<HTMLInputElement>("#settings-hf-token").disabled = busy
    for (const action of ["clear-token", "toggle-token-visibility"]) {
      this.element<HTMLButtonElement>(`[data-action="${action}"]`).disabled = busy
    }
  }

  showMessage(message: string, state: "ready" | "saving" | "error" = "ready"): void {
    const element = this.element("#settings-message")
    element.textContent = message
    element.dataset["state"] = state
  }

  private renderVisibility(visible: boolean): void {
    const button = this.element<HTMLButtonElement>("#toggle-token-visibility")
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
