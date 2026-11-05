//! A project's dictation dictionary, read from `.muxy/dictionary.txt`.
//!
//! Each line is a term the recognizer should expect, optionally followed by
//! `:` and comma-separated phrases it tends to hear instead:
//!
//! ```text
//! # Comments start with a hash.
//! Muxy: mucksy, mixie
//! GPUI: gpui, g p u i
//! Ghostty
//! ```
//!
//! Terms bias recognition. Phrases are replaced with their term after it.

pub const DIRECTORY: &str = ".muxy";
pub const FILE: &str = "dictionary.txt";
pub const PATH: &str = ".muxy/dictionary.txt";
const MAX_BYTES: usize = 64 * 1024;
const MAX_TERMS: usize = 500;
const MAX_TERM_BYTES: usize = 200;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Dictionary {
    terms: Vec<String>,
    /// Lowercase words of each heard phrase and its term, longest first.
    corrections: Vec<(Vec<String>, String)>,
}

impl Dictionary {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > MAX_BYTES {
            return Err("The dictionary exceeds the 64 KiB limit".into());
        }
        let mut dictionary = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (term, heard) = line.split_once(':').unwrap_or((line, ""));
            let term = term.trim();
            if term.is_empty() {
                continue;
            }
            if term.len() > MAX_TERM_BYTES {
                return Err(format!(
                    "Dictionary terms must be at most {MAX_TERM_BYTES} bytes"
                ));
            }
            if dictionary.terms.len() == MAX_TERMS {
                return Err(format!("The dictionary has more than {MAX_TERMS} terms"));
            }
            if !dictionary.terms.iter().any(|known| known == term) {
                dictionary.terms.push(term.to_owned());
            }
            for phrase in heard.split(',') {
                let words: Vec<_> = words(phrase)
                    .map(|(start, end)| phrase[start..end].to_lowercase())
                    .collect();
                if !words.is_empty() {
                    dictionary.corrections.push((words, term.to_owned()));
                }
            }
        }
        dictionary
            .corrections
            .sort_by_key(|(words, _)| std::cmp::Reverse(words.len()));
        Ok(dictionary)
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// The terms recognition should expect, in file order.
    pub fn terms(&self) -> &[String] {
        &self.terms
    }

    /// Replaces each heard phrase with its term. Matches whole words, ignoring
    /// case and the punctuation between words.
    pub fn correct(&self, transcript: &str) -> String {
        if self.corrections.is_empty() {
            return transcript.to_owned();
        }
        let spans: Vec<_> = words(transcript).collect();
        let lower: Vec<_> = spans
            .iter()
            .map(|&(start, end)| transcript[start..end].to_lowercase())
            .collect();
        let mut output = String::with_capacity(transcript.len());
        let mut copied = 0;
        let mut index = 0;
        while index < spans.len() {
            let matched = self.corrections.iter().find(|(words, _)| {
                lower
                    .get(index..index + words.len())
                    .is_some_and(|candidate| candidate == words.as_slice())
            });
            let Some((words, term)) = matched else {
                index += 1;
                continue;
            };
            let start = spans[index].0;
            let end = spans[index + words.len() - 1].1;
            output.push_str(&transcript[copied..start]);
            output.push_str(term);
            copied = end;
            index += words.len();
        }
        output.push_str(&transcript[copied..]);
        output
    }
}

/// Byte ranges of the runs of letters, digits and apostrophes in `text`.
fn words(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    let word = |character: char| character.is_alphanumeric() || matches!(character, '\'' | '’');
    let mut characters = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let (start, _) = characters
            .by_ref()
            .find(|&(_, character)| word(character))?;
        let mut end = text.len();
        while let Some(&(index, character)) = characters.peek() {
            if !word(character) {
                end = index;
                break;
            }
            characters.next();
        }
        Some((start, end))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heard_phrases_become_their_term_without_touching_other_words() {
        let dictionary = Dictionary::parse(
            "# project words\nMuxy: mucksy, mix e\nGPUI: g p u i, gpui\n\ncargo clippy: cargo clippie\nGhostty\n",
        )
        .unwrap();
        assert_eq!(
            dictionary.terms(),
            ["Muxy", "GPUI", "cargo clippy", "Ghostty"]
        );
        for (heard, expected) in [
            (
                "Open Mucksy, then run Cargo Clippie.",
                "Open Muxy, then run cargo clippy.",
            ),
            ("Is G.P.U.I. fast? gpui is.", "Is GPUI. fast? GPUI is."),
            ("mix everything in Mix E", "mix everything in Muxy"),
            ("mucksyy and premucksy stay", "mucksyy and premucksy stay"),
            ("Ünïcödé mucksy’s", "Ünïcödé mucksy’s"),
            ("", ""),
        ] {
            assert_eq!(dictionary.correct(heard), expected);
        }
    }

    #[test]
    fn longer_phrases_win_over_their_prefixes() {
        let dictionary = Dictionary::parse("Rust: rust\nrust-analyzer: rust analyzer").unwrap();
        assert_eq!(
            dictionary.correct("rust analyzer needs rust"),
            "rust-analyzer needs Rust"
        );
    }

    #[test]
    fn oversized_dictionaries_are_rejected() {
        assert!(Dictionary::parse(&"a\n".repeat(MAX_BYTES)).is_err());
        assert!(Dictionary::parse(&"x".repeat(MAX_TERM_BYTES + 1)).is_err());
        let many: Vec<_> = (0..=MAX_TERMS)
            .map(|index| format!("term{index}"))
            .collect();
        assert!(Dictionary::parse(&many.join("\n")).is_err());
    }
}
