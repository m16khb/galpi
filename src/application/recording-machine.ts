export interface RecordingViewState {
  readonly status: "idle" | "starting" | "recording" | "stopping" | "failed" | "completed"
  readonly recordingId: string | null
  readonly path: string | null
  readonly elapsedSeconds: number
  readonly message: string
}

export const initialRecordingState: RecordingViewState = {
  status: "idle",
  recordingId: null,
  path: null,
  elapsedSeconds: 0,
  message: "마이크로 바로 녹음할 수 있습니다.",
}

export function beginRecordingState(): RecordingViewState {
  return {
    ...initialRecordingState,
    status: "starting",
    message: "마이크 권한과 입력 장치를 확인합니다.",
  }
}

export function canStartRecording(state: RecordingViewState): boolean {
  return state.status === "idle" || state.status === "completed" || state.status === "failed"
}

export function startRecordingState(
  recordingId: string,
  path: string,
): RecordingViewState {
  return {
    status: "recording",
    recordingId,
    path,
    elapsedSeconds: 0,
    message: "녹음 중입니다.",
  }
}

export function tickRecording(state: RecordingViewState): RecordingViewState {
  if (state.status !== "recording") return state
  return { ...state, elapsedSeconds: state.elapsedSeconds + 1 }
}

export function stopRecordingState(state: RecordingViewState): RecordingViewState {
  return { ...state, status: "stopping", message: "녹음 파일을 마무리합니다." }
}

export function cancelRecordingState(state: RecordingViewState): RecordingViewState {
  return { ...state, status: "stopping", message: "녹음을 취소하고 파일을 정리합니다." }
}

export function completeRecordingState(
  state: RecordingViewState,
  path: string,
): RecordingViewState {
  return {
    ...state,
    status: "completed",
    path,
    message: "녹음이 완료되어 전사 파일로 선택했습니다.",
  }
}

export function failRecordingState(message: string): RecordingViewState {
  return { ...initialRecordingState, status: "failed", message }
}
