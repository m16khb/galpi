/**
 * Normalize an optional free-text field: trimmed, or null when nothing is left.
 *
 * Roster and glossary fields arrive from text inputs, where "absent" and
 * "whitespace the user typed and deleted" look the same. Collapsing both to
 * null keeps the stored document from carrying empty strings that later read
 * as real values.
 */
export function emptyToNull(value: string | null): string | null {
  const trimmed = value?.trim() ?? ""
  return trimmed.length > 0 ? trimmed : null
}
