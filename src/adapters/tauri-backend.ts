import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { open } from "@tauri-apps/plugin-dialog"
import { openUrl } from "@tauri-apps/plugin-opener"
import { z } from "zod"

import type { EnvironmentStatus, JobEvent, TranscriptionResult } from "../domain/job"
import type { SpeakerHint } from "../domain/speaker"

const environmentSchema = z.object({
  engineReady: z.boolean(),
  modelsReady: z.boolean(),
  ffmpegReady: z.boolean(),
  dataDirectory: z.string(),
  defaultOutputDirectory: z.string(),
  engineVersion: z.string(),
})

const transcriptionResultSchema = z.object({
  jobId: z.string(),
  srt: z.string(),
  txt: z.string(),
  checkpoint: z.string(),
  outputDirectory: z.string(),
  segments: z.number().int().nonnegative(),
  filtered: z.number().int().nonnegative(),
})

const setupResultSchema = z.object({
  jobId: z.string(),
  status: environmentSchema,
})

const recordingStatusSchema = z.object({
  recordingId: z.string(),
  path: z.string(),
  sampleRate: z.number().int().positive(),
  channels: z.number().int().positive(),
})

const recordingResultSchema = recordingStatusSchema.extend({
  frames: z.number().int().nonnegative(),
  durationSeconds: z.number().nonnegative(),
})

const recordingFailureSchema = z.object({
  recordingId: z.string(),
  code: z.string(),
  message: z.string(),
})

const rawJobEventSchema = z.discriminatedUnion("type", [
  z.object({
    jobId: z.string(),
    type: z.literal("phase"),
    phase: z.string(),
    percent: z.number(),
    message: z.string(),
  }),
  z.object({
    jobId: z.string(),
    type: z.literal("log"),
    stream: z.string(),
    message: z.string(),
  }),
  z.object({
    jobId: z.string(),
    type: z.literal("completed"),
    srt: z.string(),
    txt: z.string(),
    checkpoint: z.string(),
    segments: z.number().int().nonnegative(),
    filtered: z.number().int().nonnegative(),
  }),
  z.object({
    jobId: z.string(),
    type: z.literal("prepared"),
    engine_version: z.string(),
  }),
  z.object({
    jobId: z.string(),
    type: z.literal("error"),
    code: z.string(),
    message: z.string(),
  }),
])

export interface SetupResult {
  readonly jobId: string
  readonly status: EnvironmentStatus
}

export type RecordingStatus = z.infer<typeof recordingStatusSchema>
export type RecordingResult = z.infer<typeof recordingResultSchema>
export type RecordingFailure = z.infer<typeof recordingFailureSchema>

export interface BackendPort {
  diagnose(): Promise<EnvironmentStatus>
  prepare(huggingFaceToken: string | null): Promise<SetupResult>
  transcribe(request: {
    readonly jobId: string
    readonly inputPath: string
    readonly outputRoot: string
    readonly speakerHint: SpeakerHint
  }): Promise<TranscriptionResult>
  cancel(jobId: string): Promise<void>
  openArtifact(jobId: string, kind: "srt" | "speaker_text" | "checkpoint"): Promise<void>
  revealOutput(jobId: string): Promise<void>
  startRecording(outputRoot: string): Promise<RecordingStatus>
  stopRecording(recordingId: string): Promise<RecordingResult>
  cancelRecording(recordingId: string): Promise<void>
  listenToRecordingFailures(handler: (event: RecordingFailure) => void): Promise<UnlistenFn>
  chooseAudio(): Promise<string | null>
  chooseOutputDirectory(): Promise<string | null>
  openModelAccessPage(): Promise<void>
  listenToJobs(handler: (event: JobEvent) => void): Promise<UnlistenFn>
}

export class TauriBackend implements BackendPort {
  async diagnose(): Promise<EnvironmentStatus> {
    return environmentSchema.parse(await invoke<unknown>("diagnose_environment"))
  }

  async prepare(huggingFaceToken: string | null): Promise<SetupResult> {
    return setupResultSchema.parse(
      await invoke<unknown>("prepare_environment", {
        request: { huggingFaceToken },
      }),
    )
  }

  async transcribe(request: {
    readonly jobId: string
    readonly inputPath: string
    readonly outputRoot: string
    readonly speakerHint: SpeakerHint
  }): Promise<TranscriptionResult> {
    return transcriptionResultSchema.parse(
      await invoke<unknown>("start_transcription", { request }),
    )
  }

  async cancel(jobId: string): Promise<void> {
    await invoke("cancel_job", { jobId })
  }

  async openArtifact(
    jobId: string,
    kind: "srt" | "speaker_text" | "checkpoint",
  ): Promise<void> {
    await invoke("open_artifact", { jobId, kind })
  }

  async revealOutput(jobId: string): Promise<void> {
    await invoke("reveal_output_directory", { jobId })
  }

  async startRecording(outputRoot: string): Promise<RecordingStatus> {
    return recordingStatusSchema.parse(
      await invoke<unknown>("start_recording", { outputRoot }),
    )
  }

  async stopRecording(recordingId: string): Promise<RecordingResult> {
    return recordingResultSchema.parse(
      await invoke<unknown>("stop_recording", { recordingId }),
    )
  }

  async cancelRecording(recordingId: string): Promise<void> {
    await invoke("cancel_recording", { recordingId })
  }

  listenToRecordingFailures(handler: (event: RecordingFailure) => void): Promise<UnlistenFn> {
    return listen<unknown>("recording-event", ({ payload }) => {
      handler(recordingFailureSchema.parse(payload))
    })
  }

  async chooseAudio(): Promise<string | null> {
    const selection = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: "오디오",
          extensions: ["m4a", "mp3", "wav", "mp4", "mov", "aac", "flac", "ogg"],
        },
      ],
    })
    return typeof selection === "string" ? selection : null
  }

  async chooseOutputDirectory(): Promise<string | null> {
    const selection = await open({ multiple: false, directory: true })
    return typeof selection === "string" ? selection : null
  }

  async openModelAccessPage(): Promise<void> {
    await openUrl("https://huggingface.co/pyannote/speaker-diarization-community-1")
  }

  async listenToJobs(handler: (event: JobEvent) => void): Promise<UnlistenFn> {
    return listen<unknown>("job-event", ({ payload }) => {
      const raw = rawJobEventSchema.parse(payload)
      const event: JobEvent =
        raw.type === "prepared"
          ? {
              jobId: raw.jobId,
              type: raw.type,
              engineVersion: raw.engine_version,
            }
          : raw
      handler(event)
    })
  }
}

export function errorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = error.message
    if (typeof message === "string") {
      return message
    }
  }
  return error instanceof Error ? error.message : String(error)
}
