//! Documentation AI — search and answer questions from Kabootar docs.
//!
//! Embeds all `docs/*.md` at compile time. Optional LLM polish via `docai-llm` feature.

mod corpus;
mod search;

pub use search::{DocHit, DocIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocAnswer {
    pub text: String,
    pub sources: Vec<DocHit>,
}

/// Build the documentation index from embedded markdown files.
pub fn index() -> DocIndex {
    DocIndex::from_chunks(corpus::chunks())
}

/// Search documentation for relevant passages.
pub fn search(query: &str, limit: usize) -> Vec<DocHit> {
    index().search(query, limit)
}

/// Answer a developer question using retrieved documentation context.
pub fn ask(query: &str) -> DocAnswer {
    let hits = search(query, 5);
    let text = synthesize_answer(query, &hits);
    DocAnswer {
        text,
        sources: hits,
    }
}

/// List documentation topics (file stems).
pub fn topics() -> Vec<&'static str> {
    corpus::topics()
}

fn synthesize_answer(query: &str, hits: &[DocHit]) -> String {
    if hits.is_empty() {
        return format!(
            "Jag hittade inget i dokumentationen om \"{}\".\n\n\
             Tips: prova :topics för ämnen, eller fråga t.ex. \"import science\", \"statistik\", \"SQL WHERE\".",
            query
        );
    }

    let mut out = String::new();
    let best = &hits[0];
    out.push_str(&format!(
        "**{}** — {}\n\n",
        best.doc_title, best.heading
    ));

    if hits.len() == 1 {
        out.push_str(&trim_excerpt(&best.excerpt, 900));
    } else {
        out.push_str("Relevanta avsnitt:\n\n");
        for (i, hit) in hits.iter().take(4).enumerate() {
            out.push_str(&format!(
                "{}. **{}** ({}) — {}\n",
                i + 1,
                hit.heading,
                hit.doc_title,
                trim_excerpt(&hit.excerpt, 220)
            ));
        }
        out.push('\n');
        out.push_str("---\n\n");
        out.push_str(&format!("Mest relevant:\n\n{}", trim_excerpt(&best.excerpt, 700)));
    }

    if let Some(code) = extract_code_example(&best.excerpt) {
        out.push_str("\n\n```kabootar\n");
        out.push_str(code.trim());
        out.push_str("\n```");
    }

    out.push_str("\n\nKällor: ");
    let sources: Vec<String> = hits
        .iter()
        .take(3)
        .map(|h| format!("{}#{}", h.path, slug_heading(&h.heading)))
        .collect();
    out.push_str(&sources.join(", "));
    out
}

fn slug_heading(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn trim_excerpt(text: &str, max: usize) -> String {
    let flat: String = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        let mut s: String = flat.chars().take(max).collect();
        s.push_str("…");
        s
    }
}

fn extract_code_example(excerpt: &str) -> Option<String> {
    let mut in_block = false;
    let mut lines = Vec::new();
    for line in excerpt.lines() {
        if line.trim_start().starts_with("```") {
            if in_block {
                return Some(lines.join("\n"));
            }
            in_block = true;
            continue;
        }
        if in_block {
            lines.push(line);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_science_import() {
        let hits = search("hur importerar jag science", 3);
        assert!(!hits.is_empty());
        assert!(
            hits.iter()
                .any(|h| h.path.contains("SCIENCE") || h.excerpt.contains("import"))
        );
    }

    #[test]
    fn asks_about_statistics() {
        let answer = ask("stat_mean medelvärde");
        assert!(answer.text.to_lowercase().contains("stat") || answer.text.contains("medel"));
        assert!(!answer.sources.is_empty());
    }

    #[test]
    fn topics_include_science() {
        assert!(topics().iter().any(|t| *t == "SCIENCE"));
    }
}
