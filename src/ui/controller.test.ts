import { describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import type { BackendPort } from "../domain/backend"
import { AppView } from "./app-view"
import { AppController } from "./controller"

const styles = await Bun.file(new URL("../styles.css", import.meta.url)).text()

function unavailableBackend(): BackendPort {
  const unavailable = () => Promise.reject(new Error("no native runtime"))
  return {
    diagnose: unavailable,
    prepare: unavailable,
    loadHuggingFaceToken: unavailable,
    saveHuggingFaceToken: unavailable,
    loadAssistantSettings: unavailable,
    saveAssistantSettings: unavailable,
    refineTranscript: unavailable,
    transcribe: unavailable,
    importTranscript: unavailable,
    cancel: unavailable,
    openArtifact: unavailable,
    revealOutput: unavailable,
    startRecording: unavailable,
    stopRecording: unavailable,
    cancelRecording: unavailable,
    listenToRecordingFailures: unavailable,
    chooseAudio: unavailable,
    chooseTranscript: unavailable,
    chooseOutputDirectory: unavailable,
    openModelAccessPage: unavailable,
    listenToJobs: unavailable,
  }
}

function createHarness(): { controller: AppController; root: HTMLElement; window: Window } {
  const window = new Window()
  const sheet = window.document.createElement("style")
  sheet.textContent = styles
  window.document.head.appendChild(sheet)
  const root = window.document.createElement("div") as unknown as HTMLElement
  window.document.body.appendChild(root as unknown as never)
  const controller = new AppController(unavailableBackend(), new AppView(root))
  return { controller, root, window }
}

describe("AppController startup without a native runtime", () => {
  test("surfaces a visible error instead of a silent dead shell", async () => {
    // Given / When: every backend call, starting with the event subscription, rejects
    const { controller, root } = createHarness()
    await controller.start()

    // Then: the persistent banner carries the failure in Korean user copy
    const banner = root.querySelector("#app-error") as HTMLElement
    expect(banner.hidden).toBe(false)
    expect(banner.textContent).toContain("네이티브 런타임")
    controller.stop()
  })

  test("keeps controls bound so the settings sheet still opens", async () => {
    // Given: the native runtime never attached
    const { controller, root } = createHarness()
    await controller.start()

    // When: the user opens settings anyway
    ;(root.querySelector('[data-action="open-settings"]') as HTMLElement).click()
    await new Promise((resolve) => setTimeout(resolve, 0))
    await new Promise((resolve) => setTimeout(resolve, 0))

    // Then: the dialog opened and reports the load failure inside the sheet
    expect((root.querySelector("#settings-dialog") as HTMLElement).hidden).toBe(false)
    const message = root.querySelector("#settings-message") as HTMLElement
    expect(message.dataset["state"]).toBe("error")
    controller.stop()
  })

  test("surfaces direct IPC actions that fail instead of dying silently", async () => {
    // Given: the native runtime never attached, so every direct IPC call rejects
    const { controller, root } = createHarness()
    await controller.start()
    const banner = root.querySelector("#app-error") as HTMLElement
    const flush = () => new Promise((resolve) => setTimeout(resolve, 0))

    // When: the user triggers the three unguarded direct actions. Transcript
    // import is absent here: without a runtime the output folder is unknown and
    // the import action reports that guard error instead of reaching IPC.
    for (const action of ["choose-audio", "choose-output", "model-access"]) {
      banner.textContent = ""
      ;(root.querySelector(`[data-action="${action}"]`) as HTMLElement).click()
      await flush()
      await flush()

      // Then: each failure lands in the visible banner with user-facing copy
      expect(banner.hidden).toBe(false)
      expect(banner.textContent).toBe("예기치 못한 오류가 발생했습니다.")
    }
    controller.stop()
  })
})

describe("AppController transcript import", () => {
  test("imports an existing transcript and unlocks augmentation", async () => {
    // Given: a native runtime that can import a transcript without transcription
    const window = new Window()
    const sheet = window.document.createElement("style")
    sheet.textContent = styles
    window.document.head.appendChild(sheet)
    const root = window.document.createElement("div") as unknown as HTMLElement
    window.document.body.appendChild(root as unknown as never)
    const unavailable = () => Promise.reject(new Error("unused"))
    const backend = {
      diagnose: async () => ({
        engineReady: false,
        modelsReady: false,
        ffmpegReady: false,
        dataDirectory: "/tmp/galpi",
        defaultOutputDirectory: "/tmp/Documents/Galpi",
        engineVersion: "test",
      }),
      prepare: unavailable,
      loadHuggingFaceToken: async () => null,
      saveHuggingFaceToken: unavailable,
      loadAssistantSettings: async () => ({
        apiKey: "zai_key",
        model: null,
        baseUrl: null,
        reasoningEffort: null,
        background: null,
        participants: [],
        glossary: [],
      }),
      saveAssistantSettings: unavailable,
      refineTranscript: unavailable,
      transcribe: unavailable,
      importTranscript: async () => ({
        jobId: "job-import-1",
        txt: "/tmp/Documents/Galpi/팀미팅/팀미팅.txt",
        outputDirectory: "/tmp/Documents/Galpi/팀미팅",
      }),
      cancel: unavailable,
      openArtifact: unavailable,
      revealOutput: unavailable,
      startRecording: unavailable,
      stopRecording: unavailable,
      cancelRecording: unavailable,
      listenToRecordingFailures: async () => () => undefined,
      chooseAudio: unavailable,
      chooseTranscript: async () => "/tmp/팀미팅.txt",
      chooseOutputDirectory: unavailable,
      openModelAccessPage: unavailable,
      listenToJobs: async () => () => undefined,
    }
    const controller = new AppController(backend as unknown as BackendPort, new AppView(root))
    await controller.start()

    // When: the user imports an existing transcript
    ;(root.querySelector('[data-action="import-transcript"]') as HTMLElement).click()
    await new Promise((resolve) => setTimeout(resolve, 0))
    await new Promise((resolve) => setTimeout(resolve, 0))

    // Then: the transcript renders as the meeting result and augmentation unlocks
    expect((root.querySelector("#transcript-selection") as HTMLElement).dataset["selected"]).toBe(
      "true",
    )
    expect((root.querySelector("#result-txt") as HTMLElement).textContent).toBe(
      "/tmp/Documents/Galpi/팀미팅/팀미팅.txt",
    )
    expect((root.querySelector("#result-srt-row") as HTMLElement).hidden).toBe(true)
    expect((root.querySelector("#result-checkpoint-row") as HTMLElement).hidden).toBe(true)
    expect((root.querySelector("#refine-button") as HTMLButtonElement).disabled).toBe(false)
    controller.stop()
  })
})
