use serde::{Deserialize, Serialize};

/// One saved meeting participant reused across meetings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub team: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl Participant {
    /// Drop a nameless entry; a participant without a name cannot label a speaker.
    fn trimmed(self) -> Option<Self> {
        let name = keep_filled(Some(self.name))?;
        Some(Self {
            id: self.id,
            name,
            team: keep_filled(self.team),
            role: keep_filled(self.role),
            description: keep_filled(self.description),
            aliases: self
                .aliases
                .into_iter()
                .filter_map(|alias| keep_filled(Some(alias)))
                .collect(),
        })
    }
}

/// One glossary term applied to every refinement to correct misheard words.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntry {
    pub id: String,
    pub term: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl GlossaryEntry {
    /// Drop a termless row; a glossary corrects terms, not prose.
    fn trimmed(self) -> Option<Self> {
        let term = keep_filled(Some(self.term))?;
        Some(Self {
            id: self.id,
            term,
            description: keep_filled(self.description),
        })
    }
}

/// Assistant credentials, background context, and the saved participant roster.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSettings {
    /// Whether a key is on file, for callers that only need to know that.
    ///
    /// The key itself never travels with these settings. The sheet autosaves
    /// the whole document whenever any field changes, so a secret carried in
    /// that payload is one absent field away from being erased; `save_assistant_api_key`
    /// is the only way in or out. Reading the key is also a keychain access,
    /// and macOS turns each one into a question for the user, so the sheet
    /// asks this flag instead.
    #[serde(default)]
    pub api_key_stored: bool,
    pub model: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    pub background: Option<String>,
    #[serde(default)]
    pub participants: Vec<Participant>,
    #[serde(default)]
    pub glossary: Vec<GlossaryEntry>,
}

impl AssistantSettings {
    pub fn trimmed(self) -> Self {
        Self {
            api_key_stored: self.api_key_stored,
            model: keep_filled(self.model),
            base_url: keep_filled(self.base_url),
            reasoning_effort: self.reasoning_effort.and_then(|effort| {
                let lowered = effort.trim().to_lowercase();
                (lowered == "low" || lowered == "medium" || lowered == "high" || lowered == "max")
                    .then_some(lowered)
            }),
            background: keep_filled(self.background),
            participants: self
                .participants
                .into_iter()
                .filter_map(Participant::trimmed)
                .collect(),
            glossary: self
                .glossary
                .into_iter()
                .filter_map(GlossaryEntry::trimmed)
                .collect(),
        }
    }
}

fn keep_filled(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{AssistantSettings, GlossaryEntry, Participant};

    fn participant(name: &str, aliases: &[&str]) -> Participant {
        Participant {
            id: "person-1".to_owned(),
            name: name.to_owned(),
            team: None,
            role: None,
            description: None,
            aliases: aliases.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn trimming_drops_nameless_participants_and_blank_aliases() {
        // Given
        let settings = AssistantSettings {
            participants: vec![
                participant("  하빈  ", &["하빈이", " ", ""]),
                participant("   ", &[]),
            ],
            ..AssistantSettings::default()
        };

        // When
        let trimmed = settings.trimmed();

        // Then
        assert_eq!(trimmed.participants.len(), 1);
        assert_eq!(trimmed.participants[0].name, "하빈");
        assert_eq!(trimmed.participants[0].aliases, ["하빈이"]);
    }

    #[test]
    fn trimming_drops_termless_glossary_rows() {
        // Given
        let settings = AssistantSettings {
            glossary: vec![
                GlossaryEntry {
                    id: "term-1".to_owned(),
                    term: "갈피".to_owned(),
                    description: Some("  ".to_owned()),
                },
                GlossaryEntry {
                    id: "term-2".to_owned(),
                    term: "  ".to_owned(),
                    description: None,
                },
            ],
            ..AssistantSettings::default()
        };

        // When
        let trimmed = settings.trimmed();

        // Then
        assert_eq!(trimmed.glossary.len(), 1);
        assert_eq!(trimmed.glossary[0].term, "갈피");
        assert_eq!(trimmed.glossary[0].description, None);
    }

    #[test]
    fn trimming_normalizes_or_rejects_reasoning_effort() {
        // Given
        let supported = AssistantSettings {
            reasoning_effort: Some(" HIGH ".to_owned()),
            ..AssistantSettings::default()
        };
        let unsupported = AssistantSettings {
            reasoning_effort: Some("extreme".to_owned()),
            ..AssistantSettings::default()
        };

        // When / Then
        assert_eq!(
            supported.trimmed().reasoning_effort,
            Some("high".to_owned())
        );
        assert_eq!(unsupported.trimmed().reasoning_effort, None);
    }
}
