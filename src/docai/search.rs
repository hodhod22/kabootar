use super::corpus::DocChunk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocHit {
    pub path: String,
    pub doc_title: String,
    pub heading: String,
    pub excerpt: String,
    pub score: i32,
}

pub struct DocIndex {
    chunks: Vec<DocChunk>,
}

impl DocIndex {
    pub fn from_chunks(chunks: Vec<DocChunk>) -> Self {
        Self { chunks }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<DocHit> {
        let terms = tokenize(query);
        if terms.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let mut hits: Vec<DocHit> = self
            .chunks
            .iter()
            .map(|chunk| {
                let score = score_chunk(chunk, &terms, &query_lower);
                DocHit {
                    path: chunk.path.to_string(),
                    doc_title: chunk.title.clone(),
                    heading: chunk.heading.clone(),
                    excerpt: chunk.excerpt.clone(),
                    score,
                }
            })
            .filter(|h| h.score > 0)
            .collect();

        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.heading.cmp(&b.heading)));
        hits.truncate(limit);
        hits
    }
}

fn score_chunk(chunk: &DocChunk, terms: &[String], query_lower: &str) -> i32 {
    let mut score = 0i32;
    let heading_lower = chunk.heading.to_lowercase();
    let path_lower = chunk.path.to_lowercase();
    let title_lower = chunk.title.to_lowercase();

    if chunk.excerpt.to_lowercase().contains(query_lower) {
        score += 25;
    }
    if heading_lower.contains(query_lower) {
        score += 20;
    }

    for term in terms {
        if heading_lower.contains(term) {
            score += 12;
        }
        if title_lower.contains(term) {
            score += 8;
        }
        if path_lower.contains(term) {
            score += 6;
        }
        let count = chunk.body_lower.matches(term.as_str()).count() as i32;
        score += count * 3;
        if count > 0 && chunk.excerpt.len() < 400 {
            score += 2;
        }
    }

    // Boost common developer intents
    if query_lower.contains("import") && chunk.body_lower.contains("import") {
        score += 5;
    }
    if (query_lower.contains("exempel") || query_lower.contains("example"))
        && chunk.body_lower.contains("```")
    {
        score += 4;
    }
    if (query_lower.contains("javascript") || query_lower.contains("js-utvecklare"))
        && path_lower.contains("javascript")
    {
        score += 30;
    }

    score
}

fn tokenize(query: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "och", "att", "jag", "hur", "vad", "för", "den", "det", "som", "kan", "ska", "är", "en",
        "ett", "the", "a", "an", "how", "what", "does", "do", "is", "are", "to", "in", "on",
        "about", "fråga", "om", "med", "från", "var", "vilken", "vilka",
    ];

    query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|w| w.len() >= 2)
        .filter(|w| !STOP.contains(w))
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docai::corpus::chunks;

    #[test]
    fn tokenize_removes_stopwords() {
        let t = tokenize("hur importerar jag science modulen");
        assert!(t.contains(&"importerar".to_string()));
        assert!(t.contains(&"science".to_string()));
        assert!(!t.contains(&"hur".to_string()));
    }

    #[test]
    fn ranks_sql_above_unrelated() {
        let idx = DocIndex::from_chunks(chunks());
        let hits = idx.search("SQL INSERT parameter", 5);
        assert!(!hits.is_empty());
        assert!(hits[0].path.contains("SQL") || hits[0].excerpt.to_lowercase().contains("insert"));
    }
}
