import {
  beginRecordingState,
  canStartRecording,
  cancelRecordingState,
  completeRecordingState,
  failRecordingState,
  initialRecordingState,
  startRecordingState,
  stopRecordingState,
  tickRecording,
  type RecordingViewState,
} from "../application/recording-machine"
import {
  errorMessage,
  type BackendPort,
  type RecordingFailure,
} from "../adapters/tauri-backend"
import type { AppView } from "./app-view"

export class RecordingController {
  private state: RecordingViewState = initialRecordingState
  private timer: number | null = null
  private readonly pendingFailures = new Map<string, RecordingFailure>()

  constructor(
    private readonly backend: BackendPort,
    private readonly view: AppView,
    private readonly selectAudio: (path: string) => void,
  ) {}

  render(): void {
    this.view.setRecording(this.state)
  }

  async start(outputRoot: string | null): Promise<void> {
    if (!canStartRecording(this.state)) return
    if (outputRoot === null) {
      this.view.showError("녹음을 저장할 출력 폴더를 먼저 선택해 주세요.")
      return
    }
    this.pendingFailures.clear()
    this.state = beginRecordingState()
    this.render()
    try {
      const started = await this.backend.startRecording(outputRoot)
      this.state = startRecordingState(started.recordingId, started.path)
      this.render()
      const earlyFailure = this.pendingFailures.get(started.recordingId)
      this.pendingFailures.clear()
      if (earlyFailure !== undefined) {
        await this.cleanupFailure(earlyFailure)
        return
      }
      this.timer = window.setInterval(() => {
        this.state = tickRecording(this.state)
        this.render()
      }, 1_000)
    } catch (error) {
      this.pendingFailures.clear()
      this.state = failRecordingState(errorMessage(error))
      this.render()
    }
  }

  async stop(): Promise<void> {
    if (this.state.status !== "recording") return
    const recordingId = this.state.recordingId
    if (recordingId === null) return
    this.clearTimer()
    this.state = stopRecordingState(this.state)
    this.render()
    try {
      const result = await this.backend.stopRecording(recordingId)
      this.selectAudio(result.path)
      this.state = completeRecordingState(this.state, result.path)
    } catch (error) {
      this.state = failRecordingState(errorMessage(error))
    }
    this.render()
  }

  async cancel(): Promise<void> {
    if (this.state.status !== "recording") return
    const recordingId = this.state.recordingId
    if (recordingId === null) return
    this.clearTimer()
    this.state = cancelRecordingState(this.state)
    this.render()
    try {
      await this.backend.cancelRecording(recordingId)
      this.state = initialRecordingState
    } catch (error) {
      this.state = failRecordingState(errorMessage(error))
    }
    this.render()
  }

  async handleFailure(event: RecordingFailure): Promise<void> {
    if (this.state.status === "starting") {
      this.pendingFailures.set(event.recordingId, event)
      return
    }
    if (event.recordingId !== this.state.recordingId) return
    await this.cleanupFailure(event)
  }

  private async cleanupFailure(event: RecordingFailure): Promise<void> {
    this.clearTimer()
    this.state = cancelRecordingState({
      ...this.state,
      recordingId: event.recordingId,
    })
    this.render()
    try {
      await this.backend.cancelRecording(event.recordingId)
    } catch (error) {
      this.state = failRecordingState(
        `${event.message} 부분 녹음 파일 정리에도 실패했습니다: ${errorMessage(error)}`,
      )
      this.render()
      return
    }
    this.state = failRecordingState(event.message)
    this.render()
  }

  dispose(): void {
    this.clearTimer()
    if (this.state.recordingId !== null) {
      void this.backend
        .cancelRecording(this.state.recordingId)
        .catch((error: unknown) => this.view.showError(errorMessage(error)))
    }
  }

  private clearTimer(): void {
    if (this.timer !== null) {
      window.clearInterval(this.timer)
      this.timer = null
    }
  }
}
