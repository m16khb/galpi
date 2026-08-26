/**
 * Resolve a required element inside a view's root.
 *
 * Every view depends on its own slice of the app template; a missing selector
 * means the template and the view have drifted apart, which is a defect to
 * surface loudly rather than a null to thread through the caller.
 */
export function required<T extends HTMLElement = HTMLElement>(
  root: HTMLElement,
  selector: string,
): T {
  const element = root.querySelector<T>(selector)
  if (element === null) throw new Error(`필수 화면 요소가 없습니다: ${selector}`)
  return element
}
