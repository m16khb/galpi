import { describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import { appTemplate } from "./app-template"
import { bindTokenGuide } from "./token-guide"

describe("bindTokenGuide (real DOM)", () => {
  test("hands focus back to the trigger when the popover closes from inside", () => {
    // Given: a bound guide with its popover open (opening geometry itself is
    // browser-verified; here the close path under fix is what matters).
    // happy-dom ships its own DOM classes, so values cross via unknown.
    const window = new Window()
    const cast = (value: unknown) => value as unknown as HTMLElement
    const root = cast(window.document.createElement("div"))
    root.innerHTML = appTemplate
    window.document.body.appendChild(root as unknown as never)
    bindTokenGuide(root)
    const trigger = cast(window.document.querySelector("#token-guide-trigger"))
    const popover = cast(window.document.querySelector("#token-guide-popover"))
    const closeButton = cast(window.document.querySelector("#token-guide-close"))
    popover.hidden = false
    trigger.setAttribute("aria-expanded", "true")

    // When: focus sits on the popover's close button and it is clicked
    closeButton.focus()
    closeButton.click()

    // Then: the popover hides and focus lands on the trigger, not <body>
    expect(popover.hidden).toBeTrue()
    expect(trigger.getAttribute("aria-expanded")).toBe("false")
    expect(window.document.activeElement?.id).toBe("token-guide-trigger")
  })
})
