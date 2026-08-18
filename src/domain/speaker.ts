export type SpeakerMode = "auto" | "exact" | "range"

export type SpeakerHint =
  | { readonly mode: "auto" }
  | { readonly mode: "exact"; readonly count: number }
  | { readonly mode: "range"; readonly min: number; readonly max: number }

export interface SpeakerForm {
  readonly mode: SpeakerMode
  readonly exact: number
  readonly min: number
  readonly max: number
}

export function buildSpeakerHint(form: SpeakerForm): SpeakerHint {
  switch (form.mode) {
    case "auto":
      return { mode: "auto" }
    case "exact":
      requirePositiveInteger(form.exact, "화자 수")
      return { mode: "exact", count: form.exact }
    case "range":
      requirePositiveInteger(form.min, "최소 화자 수")
      requirePositiveInteger(form.max, "최대 화자 수")
      if (form.min > form.max) {
        throw new Error("최소 화자 수는 최대 화자 수보다 클 수 없습니다.")
      }
      return { mode: "range", min: form.min, max: form.max }
  }
}

function requirePositiveInteger(value: number, label: string): void {
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${label}는 1 이상의 정수여야 합니다.`)
  }
}
