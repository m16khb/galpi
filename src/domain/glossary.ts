export interface GlossaryEntry {
  readonly id: string
  readonly term: string
  readonly description: string | null
}

/** Drop entries a termless row would produce; the glossary corrects terms, not prose. */
export function usableGlossary(entries: readonly GlossaryEntry[]): GlossaryEntry[] {
  return entries
    .map((entry) => ({
      id: entry.id,
      term: entry.term.trim(),
      description: emptyToNull(entry.description),
    }))
    .filter((entry) => entry.term.length > 0)
}

function emptyToNull(value: string | null): string | null {
  const trimmed = value?.trim() ?? ""
  return trimmed.length > 0 ? trimmed : null
}
