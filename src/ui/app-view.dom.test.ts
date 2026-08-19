import { beforeEach, describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import type { EnvironmentStatus } from "../domain/job"
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

function hidden(root: HTMLElement, selector: string): boolean {
  return (root.querySelector(selector) as HTMLElement).hidden === true
}

describe("AppView onboarding visibility (real DOM)", () => {
  let view: AppView
  let root: HTMLElement
  let window: Window

  beforeEach(() => {
    ;({ view, root, window } = createView())
  })

  test("renders the hidden rail steps as removed, not merely flagged", () => {
    // Given / When
    view.setEnvironment(environment(true))

    // Then
    const step = root.querySelector("#step-engine") as HTMLElement
    expect(window.getComputedStyle(step as unknown as never).display).toBe("none")
  })

  test("keeps the preparation step visible while the environment is not ready", () => {
    // When
    view.setEnvironment(environment(false))

    // Then
    expect(hidden(root, "#setup-panel")).toBe(false)
    expect(hidden(root, "#step-engine")).toBe(false)
    expect(hidden(root, "#step-model")).toBe(false)
    expect(root.querySelector("#step-transcribe-index")?.textContent).toBe("03")
  })

  test("hides engine and model preparation for an already prepared user", () => {
    // When
    view.setEnvironment(environment(true))

    // Then
    expect(hidden(root, "#setup-panel")).toBe(true)
    expect(hidden(root, "#step-engine")).toBe(true)
    expect(hidden(root, "#step-model")).toBe(true)
    expect(root.querySelector("#step-transcribe-index")?.textContent).toBe("01")
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
    expect(hidden(root, "#step-engine")).toBe(false)
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
    expect(hidden(root, "#step-model")).toBe(true)
  })
})
