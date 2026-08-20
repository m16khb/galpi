import { describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import type { BackendPort } from "../adapters/tauri-backend"
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
    cancel: unavailable,
    openArtifact: unavailable,
    revealOutput: unavailable,
    startRecording: unavailable,
    stopRecording: unavailable,
    cancelRecording: unavailable,
    listenToRecordingFailures: unavailable,
    chooseAudio: unavailable,
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

    // When: the user triggers the three unguarded direct actions
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
