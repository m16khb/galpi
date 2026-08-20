import { beforeEach, describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import type { EnvironmentStatus, TranscriptionResult } from "../domain/job"
import { AppView } from "./app-view"

const styles = await Bun.file(new URL("../styles.css", import.meta.url)).text()

function createWindow(): Window {
  const window = new Window()
  const sheet = window.document.createElement("style")
  sheet.textContent = styles
  window.document.head.appendChild(sheet)
  return window
}

function createView(): { view: AppView; root: HTMLElement; window: Window } {
  const window = createWindow()
  const root = window.document.createElement("div") as unknown as HTMLElement
  window.document.body.appendChild(root as unknown as never)
  return { view: new AppView(root), root, window }
}

function environment(ready: boolean): EnvironmentStatus {
  return {
    engineReady: ready,
    modelsReady: ready,
    ffmpegReady: ready,
    dataDirectory: "/tmp/galpi",
    defaultOutputDirectory: "/tmp/galpi/out",
    engineVersion: "3.4.3",
  }
}

const result: TranscriptionResult = {
  jobId: "id",
  srt: "/tmp/out/meeting.srt",
  txt: "/tmp/out/meeting_화자별.txt",
  checkpoint: "/tmp/out/meeting.aligned.v2.json",
  outputDirectory: "/tmp/out",
  segments: 12,
  filtered: 1,
}

function hidden(root: HTMLElement, selector: string): boolean {
  return (root.querySelector(selector) as HTMLElement).hidden === true
}

function railState(root: HTMLElement, selector: string): string {
  return (root.querySelector(selector) as HTMLElement).dataset["state"] ?? ""
}

describe("AppView stage flow (real DOM)", () => {
  let view: AppView
  let root: HTMLElement

  beforeEach(() => {
    ;({ view, root } = createView())
  })

  test("keeps the preparation panel visible while the environment is not ready", () => {
    // When
    view.setEnvironment(environment(false))

    // Then: preparation is a pre-gate panel, never a rail stage
    expect(hidden(root, "#setup-panel")).toBe(false)
    expect(root.querySelectorAll(".step-list li").length).toBe(3)
    expect(root.querySelector("#step-transcribe span")?.textContent).toBe("01")
    expect(railState(root, "#step-transcribe")).toBe("pending")
    expect(railState(root, "#step-results")).toBe("pending")
    expect(railState(root, "#step-augment")).toBe("pending")
  })

  test("hides the preparation panel once the environment is ready", () => {
    // When
    view.setEnvironment(environment(true))

    // Then
    expect(hidden(root, "#setup-panel")).toBe(true)
    expect(railState(root, "#step-transcribe")).toBe("current")
  })

  test("keeps the preparation panel visible right after preparing in this session", () => {
    // Given
    view.setEnvironment(environment(false))
    view.setBusy("setup")

    // When
    view.setEnvironment(environment(true))
    view.setBusy(null)

    // Then
    expect(hidden(root, "#setup-panel")).toBe(false)
  })

  test("hides the preparation panel once a prepared user starts transcription", () => {
    // Given
    view.setEnvironment(environment(false))
    view.setBusy("setup")
    view.setEnvironment(environment(true))

    // When
    view.setBusy("transcription")

    // Then
    expect(hidden(root, "#setup-panel")).toBe(true)
  })

  test("marks stage 01 complete and stage 02 current when results render", () => {
    // Given
    view.setEnvironment(environment(true))

    // When
    view.renderResult(result)

    // Then
    expect(railState(root, "#step-transcribe")).toBe("complete")
    expect(railState(root, "#step-results")).toBe("current")
    expect(railState(root, "#step-augment")).toBe("pending")
    expect(hidden(root, "#results-panel")).toBe(false)
    expect(hidden(root, "#augment-panel")).toBe(false)
    expect(hidden(root, "#augment-waiting")).toBe(true)
    expect(hidden(root, "#result-minutes-row")).toBe(true)
  })

  test("marks stage 03 complete when minutes render", () => {
    // Given
    view.setEnvironment(environment(true))
    view.renderResult(result)

    // When
    view.renderMinutes("/tmp/out/meeting_회의록.md")

    // Then
    expect(railState(root, "#step-augment")).toBe("complete")
    expect(hidden(root, "#result-minutes-row")).toBe(false)
  })

  test("shows the waiting hint before a transcription exists", () => {
    // Given / When
    view.setEnvironment(environment(true))

    // Then
    expect(hidden(root, "#augment-waiting")).toBe(false)
  })

  test("the augment button requires a result, busyness, and a saved key", () => {
    // Given: no key saved yet
    view.setEnvironment(environment(true))

    // Then
    expect((root.querySelector("#refine-button") as HTMLButtonElement).disabled).toBe(true)

    // When: a key arrives but there is no result
    view.setAssistantKeyReady(true)

    // Then: still disabled without a transcription
    expect((root.querySelector("#refine-button") as HTMLButtonElement).disabled).toBe(true)

    // When: a result renders
    view.renderResult(result)

    // Then: enabled with key + result + idle
    expect((root.querySelector("#refine-button") as HTMLButtonElement).disabled).toBe(false)
  })

  test("the augment key hint disappears once a key is saved", () => {
    // Given
    view.setEnvironment(environment(true))

    // When
    view.setAssistantKeyReady(true)

    // Then
    expect(hidden(root, "#augment-key-hint")).toBe(true)
  })
})
