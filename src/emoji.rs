//! Unicode emoji and search names from GitHub Gemoji and Unicode CLDR 48.

use std::sync::OnceLock;

use crate::search::{Action, Kind, ResultItem};

struct Emoji {
    glyph: &'static str,
    label: &'static str,
    aliases: &'static str,
    searchable: String,
}

fn catalog() -> &'static [Emoji] {
    static EMOJI: OnceLock<Vec<Emoji>> = OnceLock::new();
    EMOJI.get_or_init(|| {
        include_str!("../assets/emoji.tsv")
            .lines()
            .map(|row| {
                let (glyph, row) = row.split_once('\t').expect("emoji and name");
                let (label, row) = row.split_once('\t').expect("name and aliases");
                let (aliases, keywords) = row.split_once('\t').expect("aliases and keywords");
                Emoji {
                    glyph,
                    label,
                    aliases,
                    searchable: format!("{label} {aliases} {keywords}")
                        .to_lowercase()
                        .replace(['_', '-'], " "),
                }
            })
            .collect()
    })
}

pub fn search(query: &str) -> Vec<ResultItem> {
    let query = query.trim().trim_end_matches(':').to_lowercase();
    let normalized = query.replace(['_', '-'], " ");
    let words: Vec<_> = normalized.split_whitespace().collect();
    let glyph = query.replace('\u{fe0f}', "");
    let mut hits: Vec<_> = catalog()
        .iter()
        .filter_map(|entry| {
            let exact_glyph = entry.glyph.chars().filter(|&c| c != '\u{fe0f}').eq(glyph.chars());
            if !exact_glyph && !words.iter().all(|word| entry.searchable.contains(word)) {
                return None;
            }
            let rank = if exact_glyph || entry.aliases.split('|').any(|alias| alias == query) {
                0
            } else if entry.label == normalized {
                1
            } else if entry.label.starts_with(&normalized) {
                2
            } else if entry.searchable.split([' ', '|', '\t']).any(|word| word == normalized) {
                3
            } else {
                4
            };
            Some((rank, entry))
        })
        .collect();
    hits.sort_by_key(|(rank, _)| *rank);
    hits.into_iter()
        .map(|(_, emoji)| {
            let alias = emoji.aliases.split('|').next().unwrap_or_default();
            ResultItem::new(
                Kind::Emoji,
                emoji.label,
                format!(":{alias}:"),
                Action::Copy(emoji.glyph.to_owned()),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_aliases_names_and_unicode_sequences() {
        for (query, expected) in [("SMILE:", "😄"), ("red heart", "❤️"), ("👩‍💻", "👩‍💻")] {
            assert_eq!(search(query)[0].action, Action::Copy(expected.to_owned()));
        }
        assert!(
            search("сердце")
                .iter()
                .any(|item| item.action == Action::Copy("❤️".into()))
        );
        assert!(search("definitelynotanemoji").is_empty());
        assert_eq!(search("").len(), 1870);
    }
}
