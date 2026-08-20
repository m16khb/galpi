import { participantLabel, retainSelection, type Participant } from "../domain/participant"

const CHIPS_SELECTOR = "#attendee-chips"

/** Per-meeting attendee selection drawn from the saved roster. */
export class ParticipantPickerView {
  private readonly root: HTMLElement
  private readonly onSelection: (count: number) => void
  private roster: readonly Participant[] = []
  private selected = new Set<string>()

  constructor(root: HTMLElement, onSelection: (count: number) => void) {
    this.root = root
    this.onSelection = onSelection
  }

  setRoster(participants: readonly Participant[]): void {
    this.roster = participants
    this.selected = new Set(retainSelection(participants, this.selected))
    this.render()
  }

  selectedIds(): string[] {
    return retainSelection(this.roster, this.selected)
  }

  clear(): void {
    this.selected.clear()
    this.render()
    this.onSelection(0)
  }

  setBusy(busy: boolean): void {
    for (const input of this.root.querySelectorAll<HTMLInputElement>(".participant-chip input")) {
      input.disabled = busy
    }
  }

  private render(): void {
    const chips = this.element(CHIPS_SELECTOR)
    chips.replaceChildren()
    for (const participant of this.roster) {
      chips.append(this.buildChip(participant))
    }
    const empty = this.roster.length === 0
    this.element("#attendee-empty").hidden = !empty
    chips.hidden = empty
    this.refreshCount()
  }

  private buildChip(participant: Participant): HTMLElement {
    const document = this.root.ownerDocument
    const label = document.createElement("label")
    label.className = "participant-chip"
    const input = document.createElement("input")
    input.type = "checkbox"
    input.value = participant.id
    input.checked = this.selected.has(participant.id)
    input.addEventListener("change", () => {
      if (input.checked) {
        this.selected.add(participant.id)
      } else {
        this.selected.delete(participant.id)
      }
      label.dataset["selected"] = String(input.checked)
      this.refreshCount()
      this.onSelection(this.selected.size)
    })
    const mark = document.createElement("i")
    mark.className = "ph ph-check"
    mark.setAttribute("aria-hidden", "true")
    const text = document.createElement("span")
    text.textContent = participantLabel(participant)
    label.dataset["selected"] = String(input.checked)
    label.append(input, mark, text)
    return label
  }

  private refreshCount(): void {
    const count = this.selected.size
    this.element("#attendee-count").textContent =
      this.roster.length === 0 ? "명부 없음" : `${count}명 선택`
    this.element<HTMLButtonElement>("#attendee-clear").hidden = count === 0
  }

  private element<T extends HTMLElement = HTMLElement>(selector: string): T {
    const element = this.root.querySelector<T>(selector)
    if (element === null) throw new Error(`필수 화면 요소가 없습니다: ${selector}`)
    return element
  }
}
