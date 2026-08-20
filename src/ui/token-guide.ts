export function bindTokenGuide(root: HTMLElement): void {
  const trigger = required<HTMLButtonElement>(root, "#token-guide-trigger")
  const popover = required<HTMLElement>(root, "#token-guide-popover")
  const closeButton = required<HTMLButtonElement>(root, "#token-guide-close")

  const close = (): void => {
    // If focus sits inside the popover (e.g. the close button that is about
    // to disappear), hand it back to the trigger instead of dropping to
    // <body>.
    const active = root.ownerDocument.activeElement
    if (active !== null && popover.contains(active)) trigger.focus()
    popover.hidden = true
    trigger.setAttribute("aria-expanded", "false")
  }
  const open = (): void => {
    const triggerRect = trigger.getBoundingClientRect()
    const width = Math.min(420, window.innerWidth - 40)
    const left = Math.max(20, Math.min(triggerRect.right - width, window.innerWidth - width - 20))
    const top = Math.min(triggerRect.bottom + 8, window.innerHeight - 180)
    popover.style.left = `${left}px`
    popover.style.top = `${top}px`
    popover.style.setProperty("--popover-top", `${top}px`)
    popover.hidden = false
    trigger.setAttribute("aria-expanded", "true")
  }

  trigger.addEventListener("click", () => {
    if (popover.hidden) open()
    else close()
  })
  closeButton.addEventListener("click", close)
  root.addEventListener("click", (event) => {
    const target = event.target
    if (
      target instanceof Node &&
      !popover.hidden &&
      !popover.contains(target) &&
      !trigger.contains(target)
    ) {
      close()
    }
  })
  root.ownerDocument.addEventListener("keydown", (event) => {
    if (event.key === "Escape") close()
  })
  required(root, ".workspace-body").addEventListener("scroll", close)
}

function required<T extends Element>(root: HTMLElement, selector: string): T {
  const element = root.querySelector<T>(selector)
  if (element === null) throw new Error(`필수 토큰 안내 요소를 찾지 못했습니다: ${selector}`)
  return element
}
