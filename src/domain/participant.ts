export interface Participant {
  readonly id: string
  readonly name: string
  readonly team: string | null
  readonly role: string | null
  readonly description: string | null
  readonly aliases: readonly string[]
}

/** Aliases are edited as one comma-separated field and stored as a list. */
export function parseAliases(value: string): string[] {
  return value
    .split(",")
    .map((alias) => alias.trim())
    .filter((alias) => alias.length > 0)
}

export function formatAliases(aliases: readonly string[]): string {
  return aliases.join(", ")
}

export function participantLabel(participant: Participant): string {
  const detail = [participant.team, participant.role]
    .map((part) => part?.trim() ?? "")
    .filter((part) => part.length > 0)
    .join(" · ")
  return detail.length === 0 ? participant.name : `${participant.name} · ${detail}`
}

/** Drop entries a nameless row would produce; a participant without a name cannot label a speaker. */
export function usableParticipants(participants: readonly Participant[]): Participant[] {
  return participants
    .map((participant) => ({
      id: participant.id,
      name: participant.name.trim(),
      team: emptyToNull(participant.team),
      role: emptyToNull(participant.role),
      description: emptyToNull(participant.description),
      aliases: participant.aliases.map((alias) => alias.trim()).filter((alias) => alias.length > 0),
    }))
    .filter((participant) => participant.name.length > 0)
}

/** Keep only ids that still exist in the roster, in roster order. */
export function retainSelection(
  participants: readonly Participant[],
  selected: ReadonlySet<string>,
): string[] {
  return participants
    .filter((participant) => selected.has(participant.id))
    .map((participant) => participant.id)
}

function emptyToNull(value: string | null): string | null {
  const trimmed = value?.trim() ?? ""
  return trimmed.length > 0 ? trimmed : null
}
