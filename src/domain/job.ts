import type { Participant } from "./participant"

export interface EnvironmentStatus {
  readonly engineReady: boolean
  readonly modelsReady: boolean
  readonly ffmpegReady: boolean
  readonly dataDirectory: string
  readonly defaultOutputDirectory: string
  readonly engineVersion: string
}

export interface TranscriptionResult {
  readonly jobId: string
  readonly srt: string
  readonly txt: string
  readonly checkpoint: string
  readonly outputDirectory: string
  readonly segments: number
  readonly filtered: number
}

export interface AssistantSettings {
  readonly apiKey: string | null
  readonly model: string | null
  readonly background: string | null
  readonly participants: readonly Participant[]
}

export interface RefinementResult {
  readonly jobId: string
  readonly minutes: string
}

export type JobEvent =
  | {
      readonly jobId: string
      readonly type: "phase"
      readonly phase: string
      readonly percent: number
      readonly message: string
    }
  | {
      readonly jobId: string
      readonly type: "log"
      readonly stream: string
      readonly message: string
    }
  | {
      readonly jobId: string
      readonly type: "completed"
      readonly srt: string
      readonly txt: string
      readonly checkpoint: string
      readonly segments: number
      readonly filtered: number
    }
  | {
      readonly jobId: string
      readonly type: "prepared"
      readonly engineVersion: string
    }
  | {
      readonly jobId: string
      readonly type: "refined"
      readonly minutes: string
    }
  | {
      readonly jobId: string
      readonly type: "error"
      readonly code: string
      readonly message: string
    }
