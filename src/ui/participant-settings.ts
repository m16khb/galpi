import {
  formatAliases,
  parseAliases,
  usableParticipants,
  type Participant,
} from "../domain/participant"

const ROWS_SELECTOR = "#participant-rows"

/** Roster editor: one saved participant per row, saved with the rest of the settings sheet. */
export class ParticipantSettingsView {
  private readonly root: HTMLElement
  private readonly onChange: () => void

  constructor(root: HTMLElement, onChange: () => void) {
    this.root = root
    this.onChange = onChange
    this.element(ROWS_SELECTOR).addEventListener("input", () => this.refreshCount())
  }

  setRoster(participants: readonly Participant[]): void {
    const rows = this.element(ROWS_SELECTOR)
    rows.replaceChildren()
    for (const participant of participants) {
      rows.append(this.buildRow(participant))
    }
    this.refreshCount()
  }

  roster(): Participant[] {
    return usableParticipants(
      [...this.root.querySelectorAll<HTMLElement>(".participant-row")].map((row) => ({
        id: row.dataset["participantId"] ?? crypto.randomUUID(),
        name: this.field(row, ".participant-name").value,
        role: this.field(row, ".participant-role").value,
        aliases: parseAliases(this.field(row, ".participant-aliases").value),
      })),
    )
  }

  addRow(): void {
    const row = this.buildRow({
      id: crypto.randomUUID(),
      name: "",
      role: null,
      aliases: [],
    })
    this.element(ROWS_SELECTOR).append(row)
    this.refreshCount()
    row.querySelector<HTMLInputElement>(".participant-name")?.focus()
  }

  setBusy(busy: boolean): void {
    for (const input of this.root.querySelectorAll<HTMLInputElement>(".participant-row input")) {
      input.disabled = busy
    }
    this.element<HTMLButtonElement>('[data-action="add-participant"]').disabled = busy
  }

  private buildRow(participant: Participant): HTMLElement {
    const document = this.root.ownerDocument
    const row = document.createElement("div")
    row.className = "participant-row"
    row.dataset["participantId"] = participant.id
    row.append(
      this.buildInput("participant-name", "이름", participant.name),
      this.buildInput("participant-role", "역할 (선택)", participant.role ?? ""),
      this.buildInput("participant-aliases", "별칭 (쉼표로 구분)", formatAliases(participant.aliases)),
    )
    const remove = document.createElement("button")
    remove.type = "button"
    remove.className = "participant-remove"
    remove.setAttribute("aria-label", `${participant.name || "참석자"} 삭제`)
    remove.innerHTML = '<i class="ph ph-x"></i>'
    remove.addEventListener("click", () => {
      row.remove()
      this.refreshCount()
    })
    row.append(remove)
    return row
  }

  private buildInput(className: string, placeholder: string, value: string): HTMLInputElement {
    const input = this.root.ownerDocument.createElement("input")
    input.type = "text"
    input.className = className
    input.placeholder = placeholder
    input.value = value
    input.autocomplete = "off"
    input.spellcheck = false
    input.setAttribute("aria-label", placeholder)
    return input
  }

  private refreshCount(): void {
    const named = this.roster().length
    const state = this.element("#participants-count-state")
    state.textContent = named === 0 ? "비어 있음" : `${named}명`
    state.dataset["state"] = named === 0 ? "pending" : "ready"
    this.element("#participant-rows-empty").hidden =
      this.root.querySelectorAll(".participant-row").length > 0
    this.onChange()
  }

  private field(row: HTMLElement, selector: string): HTMLInputElement {
    const input = row.querySelector<HTMLInputElement>(selector)
    if (input === null) throw new Error(`참석자 입력란이 없습니다: ${selector}`)
    return input
  }

  private element<T extends HTMLElement = HTMLElement>(selector: string): T {
    const element = this.root.querySelector<T>(selector)
    if (element === null) throw new Error(`필수 화면 요소가 없습니다: ${selector}`)
    return element
  }
}
