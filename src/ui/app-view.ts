import type { JobViewState } from "../application/job-machine"
import type { EnvironmentStatus, TranscriptionResult } from "../domain/job"
import type { SpeakerForm, SpeakerMode } from "../domain/speaker"
import type { RecordingViewState } from "../application/recording-machine"
import { appTemplate } from "./app-template"
import { AssistantSettingsView } from "./assistant-settings"
import { bindTokenGuide } from "./token-guide"
import { TokenSettingsView } from "./token-settings"

export type BusyKind = "setup" | "transcription" | "refinement" | null

export class AppView {
  readonly root: HTMLElement
  readonly tokenSettings: TokenSettingsView
  readonly assistantSettings: AssistantSettingsView
  private engineReady = false
  private jobBusy = false
  private jobKind: BusyKind = null
  private recordingActive = false
  private hasResult = false

  constructor(root: HTMLElement) {
    this.root = root
    this.root.innerHTML = appTemplate
    this.tokenSettings = new TokenSettingsView(root)
    this.assistantSettings = new AssistantSettingsView(root)
    bindTokenGuide(this.root)
  }

  on(action: string, handler: () => void): void {
    for (const element of this.root.querySelectorAll<HTMLElement>(
      `[data-action="${action}"]`,
    )) {
      element.addEventListener("click", handler)
    }
  }

  onSpeakerMode(handler: (mode: SpeakerMode) => void): void {
    for (const input of this.root.querySelectorAll<HTMLInputElement>(
      'input[name="speaker-mode"]',
    )) {
      input.addEventListener("change", () => {
        if (input.checked) handler(input.value as SpeakerMode)
      })
    }
  }

  speakerForm(): SpeakerForm {
    const checked = this.root.querySelector<HTMLInputElement>(
      'input[name="speaker-mode"]:checked',
    )
    return {
      mode: (checked?.value ?? "auto") as SpeakerMode,
      exact: this.numberValue("#exact-speakers"),
      min: this.numberValue("#min-speakers"),
      max: this.numberValue("#max-speakers"),
    }
  }

  setEnvironment(status: EnvironmentStatus): void {
    this.statusRow("#engine-check", status.engineReady, `WhisperX ${status.engineVersion}`)
    this.statusRow("#model-check", status.modelsReady, "전사·정렬·화자분리 모델")
    this.statusRow("#ffmpeg-check", status.ffmpegReady, "내장 ffmpeg")
    const ready = status.engineReady && status.modelsReady && status.ffmpegReady
    this.engineReady = ready
    this.element("#setup-state").textContent = ready ? "준비 완료" : "설정 필요"
    this.element("#setup-state").dataset["state"] = ready ? "ready" : "pending"
    this.element<HTMLButtonElement>("#prepare-button").textContent = ready
      ? "준비 상태 다시 확인"
      : "로컬 엔진 준비"
    this.element("#step-engine").dataset["state"] = ready ? "complete" : "current"
    this.element("#step-model").dataset["state"] = status.modelsReady ? "complete" : "pending"
    this.element("#step-transcribe").dataset["state"] = ready ? "current" : "pending"
    this.element("#engine-version").textContent = `WhisperX ${status.engineVersion}`
    this.applyOnboarding()
    this.refreshActions()
  }

  setAudio(path: string): void {
    this.path("#audio-path", path)
    this.element("#audio-selection").dataset["selected"] = "true"
  }

  setOutput(path: string): void {
    this.path("#output-path", path)
  }

  setSpeakerMode(mode: SpeakerMode): void {
    this.element("#exact-fields").hidden = mode !== "exact"
    this.element("#range-fields").hidden = mode !== "range"
  }

  setBusy(kind: BusyKind): void {
    const busy = kind !== null
    this.jobBusy = busy
    if (kind !== null) {
      this.jobKind = kind
      this.element("#setup-progress-panel").hidden = kind !== "setup"
      this.element("#job-panel").hidden = kind === "setup"
      this.element("#job-phase-list").hidden = kind === "refinement"
    }
    this.element<HTMLButtonElement>("#cancel-button").hidden =
      !busy || this.jobKind === "setup"
    this.element<HTMLButtonElement>("#setup-cancel-button").hidden =
      !busy || this.jobKind !== "setup"
    this.element("#busy-label").textContent = busyLabel(kind)
    this.applyOnboarding()
    this.refreshActions()
  }

  setRecording(state: RecordingViewState): void {
    this.recordingActive =
      state.status === "starting" || state.status === "recording" || state.status === "stopping"
    this.element("#recorder").dataset["state"] = state.status
    this.element<HTMLButtonElement>("#record-button").hidden = this.recordingActive
    this.element("#recording-active").hidden = !this.recordingActive
    this.element("#recording-status").textContent = state.message
    this.element("#recording-label").textContent =
      state.status === "starting"
        ? "마이크 연결 중"
        : state.status === "stopping"
          ? "저장 중"
          : "녹음 중"
    this.element("#recording-path").textContent = state.path ?? "마이크 입력을 저장합니다."
    this.element("#recording-time").textContent = formatElapsed(state.elapsedSeconds)
    this.element<HTMLButtonElement>("#stop-recording-button").disabled =
      state.status === "starting" || state.status === "stopping"
    this.element<HTMLButtonElement>("#cancel-recording-button").disabled =
      state.status === "starting" || state.status === "stopping"
    this.refreshActions()
  }

  renderJob(state: JobViewState): void {
    this.element("#setup-progress-panel").hidden =
      state.status === "idle" || this.jobKind !== "setup"
    this.element("#job-panel").hidden = state.status === "idle" || this.jobKind === "setup"
    this.element("#job-message").textContent = state.message
    this.element("#setup-job-message").textContent = state.message
    this.element("#job-percent").textContent = `${Math.round(state.percent)}%`
    this.element("#setup-job-percent").textContent = `${Math.round(state.percent)}%`
    this.renderProgress("#job-progress", state.percent)
    this.renderProgress("#setup-job-progress", state.percent)
    for (const item of this.root.querySelectorAll<HTMLElement>("[data-phase]")) {
      item.dataset["state"] = phaseState(item.dataset["phase"] ?? "", state.phase)
    }
    for (const item of this.root.querySelectorAll<HTMLElement>("[data-setup-phase]")) {
      item.dataset["state"] = phaseState(item.dataset["setupPhase"] ?? "", state.phase)
    }
    this.element("#log-output").textContent = state.logs.join("\n")
    this.element("#setup-log-output").textContent = state.logs.join("\n")
    this.element("#error-message").textContent = state.error ?? ""
    this.element("#error-message").hidden = state.error === null
    this.element("#setup-error-message").textContent = state.error ?? ""
    this.element("#setup-error-message").hidden = state.error === null
  }

  renderResult(result: TranscriptionResult): void {
    this.element("#results-panel").hidden = false
    this.element("#result-summary").textContent =
      `${result.segments}개 발화 보존 · ${result.filtered}개 환각 제거`
    this.path("#result-srt", result.srt)
    this.path("#result-txt", result.txt)
    this.path("#result-checkpoint", result.checkpoint)
    this.element("#result-minutes-row").hidden = true
    this.element("#step-transcribe").dataset["state"] = "complete"
    this.hasResult = true
    this.refreshActions()
  }

  renderMinutes(minutes: string): void {
    this.element("#results-panel").hidden = false
    this.element("#result-minutes-row").hidden = false
    this.path("#result-minutes", minutes)
  }

  showError(message: string): void {
    this.element("#error-message").textContent = message
    this.element("#error-message").hidden = false
  }

  element<T extends HTMLElement = HTMLElement>(selector: string): T {
    const element = this.root.querySelector<T>(selector)
    if (element === null) {
      throw new Error(`필수 UI 요소를 찾지 못했습니다: ${selector}`)
    }
    return element
  }

  private numberValue(selector: string): number {
    return Number.parseInt(this.element<HTMLInputElement>(selector).value, 10)
  }

  private statusRow(selector: string, ready: boolean, label: string): void {
    const row = this.element(selector)
    row.dataset["state"] = ready ? "ready" : "pending"
    row.querySelector<HTMLElement>("[data-status-label]")?.replaceChildren(label)
    row.querySelector<HTMLElement>("[data-status-value]")?.replaceChildren(
      ready ? "준비됨" : "대기",
    )
  }

  private path(selector: string, value: string): void {
    const element = this.element(selector)
    element.textContent = value
    element.setAttribute("title", value)
  }

  private renderProgress(selector: string, percent: number): void {
    const progress = this.element<HTMLElement>(selector)
    progress.setAttribute("aria-valuenow", String(Math.round(percent)))
    progress.style.setProperty("--progress", `${Math.max(0, Math.min(100, percent))}%`)
  }

  private applyOnboarding(): void {
    // 준비가 끝난 사용자에게는 엔진·모델 준비 단계를 감추고 남은 단계 번호를 앞당긴다.
    // 이번 세션에서 준비를 직접 실행했다면 완료 메시지를 볼 수 있도록 그대로 둔다.
    const onboarded = this.engineReady && this.jobKind !== "setup"
    this.element("#setup-panel").hidden = onboarded
    this.element("#step-engine").hidden = onboarded
    this.element("#step-model").hidden = onboarded
    this.element("#step-transcribe-index").textContent = onboarded ? "01" : "03"
    this.element("#transcription-index").textContent = onboarded ? "01 / 전사" : "02 / 전사"
    this.element("#results-index").textContent = onboarded ? "02 / 완료" : "03 / 완료"
  }

  private refreshActions(): void {
    this.element<HTMLButtonElement>("#prepare-button").disabled =
      this.jobBusy || this.recordingActive
    this.element<HTMLButtonElement>("#start-button").disabled =
      !this.engineReady || this.jobBusy || this.recordingActive
    this.element<HTMLButtonElement>("#record-button").disabled = this.jobBusy
    this.element<HTMLButtonElement>("#audio-selection").disabled =
      this.jobBusy || this.recordingActive
    this.element<HTMLButtonElement>("#output-button").disabled =
      this.jobBusy || this.recordingActive
    this.element<HTMLButtonElement>("#refine-button").disabled =
      !this.hasResult || this.jobBusy || this.recordingActive
  }
}

function busyLabel(kind: BusyKind): string {
  if (kind === "setup") return "로컬 환경 준비 중"
  if (kind === "transcription") return "회의 전사 중"
  return kind === "refinement" ? "회의록 만드는 중" : ""
}

const phaseOrder = [
  "engine",
  "models",
  "transcribing",
  "aligning",
  "diarizing",
  "refining",
  "writing",
  "ready",
]

function phaseState(phase: string, current: string): string {
  const phaseIndex = phaseOrder.indexOf(phase)
  const currentIndex = phaseOrder.indexOf(current)
  if (phaseIndex < currentIndex) return "complete"
  if (phaseIndex === currentIndex) return "current"
  return "pending"
}

function formatElapsed(seconds: number): string {
  const minutes = Math.floor(seconds / 60)
  const remainder = seconds % 60
  return `${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`
}
