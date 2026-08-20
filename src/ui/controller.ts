import type { UnlistenFn } from "@tauri-apps/api/event"
import { type ArtifactKind, type BackendPort, errorMessage } from "../adapters/tauri-backend"
import {
  beginJob,
  completeJob,
  failJob,
  initialJobState,
  type JobViewState,
  reduceJobEvent,
} from "../application/job-machine"
import type { TranscriptionResult } from "../domain/job"
import { buildSpeakerHint, type SpeakerHint } from "../domain/speaker"
import type { AppView } from "./app-view"
import { RecordingController } from "./recording-controller"

export class AppController {
  private readonly backend: BackendPort
  private readonly view: AppView
  private audioPath: string | null = null
  private outputRoot: string | null = null
  private job: JobViewState = initialJobState
  private lastResult: TranscriptionResult | null = null
  private readonly recording: RecordingController
  private unlisten: UnlistenFn | null = null
  private unlistenRecording: UnlistenFn | null = null

  constructor(backend: BackendPort, view: AppView) {
    this.backend = backend
    this.view = view
    this.recording = new RecordingController(backend, view, (path) => {
      this.audioPath = path
      this.view.setAudio(path)
    })
  }

  async start(): Promise<void> {
    this.unlisten = await this.backend.listenToJobs((event) => {
      this.job = reduceJobEvent(this.job, event)
      this.view.renderJob(this.job)
    })
    this.unlistenRecording = await this.backend.listenToRecordingFailures((event) => {
      void this.recording.handleFailure(event)
    })
    this.bind()
    this.recording.render()
    try {
      const environment = await this.backend.diagnose()
      this.outputRoot = environment.defaultOutputDirectory
      this.view.setOutput(environment.defaultOutputDirectory)
      this.view.setEnvironment(environment)
      this.view.tokenSettings.setConfigured((await this.backend.loadHuggingFaceToken()) !== null)
      const assistant = await this.backend.loadAssistantSettings()
      this.view.assistantSettings.setConfigured(assistant.apiKey !== null)
      this.view.setAssistantKeyReady(assistant.apiKey !== null)
      this.view.participantSettings.setRoster(assistant.participants)
      this.view.glossarySettings.setEntries(assistant.glossary)
      this.view.attendees.setRoster(assistant.participants)
    } catch (error) {
      this.view.showError(errorMessage(error))
    }
  }

  stop(): void {
    this.recording.dispose()
    this.unlisten?.()
    this.unlisten = null
    this.unlistenRecording?.()
    this.unlistenRecording = null
  }

  private bind(): void {
    this.view.on("prepare", () => void this.prepare())
    this.view.on("open-settings", () => void this.openSettings())
    this.view.on("close-settings", () => this.view.tokenSettings.close())
    this.view.on("toggle-token-visibility", () => this.view.tokenSettings.toggleVisibility())
    this.view.on("toggle-assistant-visibility", () =>
      this.view.assistantSettings.toggleVisibility(),
    )
    this.view.on("save-token", () => void this.saveSettings())
    this.view.on("add-participant", () => this.view.participantSettings.addRow())
    this.view.on("add-glossary-entry", () => this.view.glossarySettings.addRow())
    this.view.on("clear-attendees", () => this.view.attendees.clear())
    this.view.on("clear-token", () => void this.clearToken())
    this.view.on("refine", () => void this.refine())
    this.view.on("open-minutes", () => void this.openArtifact("minutes"))
    this.view.on("model-access", () => void this.backend.openModelAccessPage())
    this.view.on("choose-audio", () => void this.chooseAudio())
    this.view.on("choose-output", () => void this.chooseOutput())
    this.view.on("transcribe", () => void this.transcribe())
    this.view.on("record", () => void this.recording.start(this.outputRoot))
    this.view.on("stop-recording", () => void this.recording.stop())
    this.view.on("cancel-recording", () => void this.recording.cancel())
    this.view.on("cancel", () => void this.cancel())
    this.view.on("open-srt", () => void this.openArtifact("srt"))
    this.view.on("open-txt", () => void this.openArtifact("speaker_text"))
    this.view.on("open-checkpoint", () => void this.openArtifact("checkpoint"))
    this.view.on("reveal-output", () => void this.revealOutput())
    this.view.onSpeakerMode((mode) => this.view.setSpeakerMode(mode))
  }

  private async prepare(): Promise<void> {
    this.begin("setup", "로컬 엔진과 모델을 준비합니다.")
    try {
      const result = await this.backend.prepare()
      this.view.setEnvironment(result.status)
      this.job = {
        ...this.job,
        status: "completed",
        jobId: result.jobId,
        phase: "ready",
        percent: 100,
        message: "로컬 전사 환경이 준비되었습니다.",
      }
      this.view.renderJob(this.job)
    } catch (error) {
      this.handleFailure(error)
    } finally {
      this.view.setBusy(null)
    }
  }

  private async openSettings(): Promise<void> {
    const settings = this.view.tokenSettings
    const assistant = this.view.assistantSettings
    settings.show()
    settings.setBusy(true)
    assistant.setBusy(true)
    settings.showMessage("저장된 설정을 불러오는 중입니다.")
    try {
      settings.setToken(await this.backend.loadHuggingFaceToken())
      const loaded = await this.backend.loadAssistantSettings()
      assistant.setSettings(loaded)
      this.view.setAssistantKeyReady(loaded.apiKey !== null)
      this.view.participantSettings.setRoster(loaded.participants)
      this.view.glossarySettings.setEntries(loaded.glossary)
      settings.showMessage("")
    } catch (error) {
      settings.showMessage(errorMessage(error), true)
    } finally {
      settings.setBusy(false)
      assistant.setBusy(false)
    }
  }

  private async saveSettings(): Promise<void> {
    const settings = this.view.tokenSettings
    const assistant = this.view.assistantSettings
    const token = settings.token().trim()
    settings.setBusy(true)
    assistant.setBusy(true)
    try {
      await this.backend.saveHuggingFaceToken(token)
      settings.setToken(token.length > 0 ? token : null)
      const saved = {
        ...assistant.settings(),
        participants: this.view.participantSettings.roster(),
        glossary: this.view.glossarySettings.entries(),
      }
      await this.backend.saveAssistantSettings(saved)
      assistant.setSettings(saved)
      this.view.setAssistantKeyReady(saved.apiKey !== null)
      this.view.participantSettings.setRoster(saved.participants)
      this.view.glossarySettings.setEntries(saved.glossary)
      this.view.attendees.setRoster(saved.participants)
      settings.showMessage("설정을 저장했습니다.")
    } catch (error) {
      settings.showMessage(errorMessage(error), true)
    } finally {
      settings.setBusy(false)
      assistant.setBusy(false)
    }
  }

  private async clearToken(): Promise<void> {
    const settings = this.view.tokenSettings
    settings.setBusy(true)
    try {
      await this.backend.saveHuggingFaceToken("")
      settings.clearToken()
      settings.showMessage("저장된 토큰을 지웠습니다.")
    } catch (error) {
      settings.showMessage(errorMessage(error), true)
    } finally {
      settings.setBusy(false)
    }
  }

  private async chooseAudio(): Promise<void> {
    const selected = await this.backend.chooseAudio()
    if (selected !== null) {
      this.audioPath = selected
      this.view.setAudio(selected)
    }
  }

  private async chooseOutput(): Promise<void> {
    const selected = await this.backend.chooseOutputDirectory()
    if (selected !== null) {
      this.outputRoot = selected
      this.view.setOutput(selected)
    }
  }

  private async transcribe(): Promise<void> {
    if (this.audioPath === null) {
      this.view.showError("먼저 전사할 오디오 파일을 선택해 주세요.")
      return
    }
    if (this.outputRoot === null) {
      this.view.showError("결과를 저장할 폴더를 선택해 주세요.")
      return
    }

    let speakerHint: SpeakerHint
    try {
      speakerHint = buildSpeakerHint(this.view.speakerForm())
    } catch (error) {
      this.view.showError(errorMessage(error))
      return
    }

    const jobId = crypto.randomUUID()
    this.begin("transcription", "오디오를 분석할 준비를 합니다.", jobId)
    try {
      const result = await this.backend.transcribe({
        jobId,
        inputPath: this.audioPath,
        outputRoot: this.outputRoot,
        speakerHint,
      })
      this.lastResult = result
      this.job = completeJob(this.job, result)
      this.view.renderJob(this.job)
      this.view.renderResult(result)
    } catch (error) {
      this.handleFailure(error)
    } finally {
      this.view.setBusy(null)
    }
  }

  private async cancel(): Promise<void> {
    if (this.job.jobId === null) {
      this.view.showError("취소할 작업 ID를 아직 받지 못했습니다.")
      return
    }
    try {
      await this.backend.cancel(this.job.jobId)
      this.job = failJob(this.job, "작업 취소를 요청했습니다.")
      this.view.renderJob(this.job)
    } catch (error) {
      this.view.showError(errorMessage(error))
    }
  }

  private async refine(): Promise<void> {
    const result = this.lastResult
    if (result === null) {
      this.view.showError("먼저 전사를 완료해 주세요.")
      return
    }
    this.begin("refinement", "저장한 사전 정보로 회의록을 만듭니다.")
    try {
      const refined = await this.backend.refineTranscript(
        result.jobId,
        this.view.attendees.selectedIds(),
      )
      this.job = {
        ...this.job,
        status: "completed",
        jobId: refined.jobId,
        phase: "writing",
        percent: 100,
        message: "회의록을 저장했습니다.",
      }
      this.view.renderJob(this.job)
      this.view.renderMinutes(refined.minutes)
    } catch (error) {
      this.handleFailure(error)
    } finally {
      this.view.setBusy(null)
    }
  }

  private async openArtifact(kind: ArtifactKind): Promise<void> {
    const jobId = this.lastResult?.jobId
    if (jobId === undefined) return
    try {
      await this.backend.openArtifact(jobId, kind)
    } catch (error) {
      this.view.showError(errorMessage(error))
    }
  }

  private async revealOutput(): Promise<void> {
    const jobId = this.lastResult?.jobId
    if (jobId === undefined) return
    try {
      await this.backend.revealOutput(jobId)
    } catch (error) {
      this.view.showError(errorMessage(error))
    }
  }

  private begin(
    kind: "setup" | "transcription" | "refinement",
    message: string,
    jobId: string | null = null,
  ): void {
    this.job = beginJob(message, jobId)
    this.view.renderJob(this.job)
    this.view.setBusy(kind)
  }

  private handleFailure(error: unknown): void {
    this.job = failJob(this.job, errorMessage(error))
    this.view.renderJob(this.job)
  }
}
