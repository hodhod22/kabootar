use super::snippets::{CodeUtil, UTILS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilHit {
    pub id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub score: i32,
}

pub fn suggest(query: &str, limit: usize) -> Vec<UtilHit> {
    let terms = tokenize(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let mut hits: Vec<UtilHit> = UTILS
        .iter()
        .map(|u| {
            let score = score_util(u, &terms, &query_lower);
            UtilHit {
                id: u.id.to_string(),
                category: u.category.to_string(),
                title: u.title.to_string(),
                description: u.description.to_string(),
                score,
            }
        })
        .filter(|h| h.score > 0)
        .collect();

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
    hits.truncate(limit);
    hits
}

fn score_util(util: &CodeUtil, terms: &[String], query_lower: &str) -> i32 {
    let mut score = 0i32;
    let id_lower = util.id.to_lowercase();
    let title_lower = util.title.to_lowercase();
    let desc_lower = util.description.to_lowercase();
    let cat_lower = util.category.to_lowercase();

    if id_lower == query_lower || id_lower.replace('-', "") == query_lower.replace(['-', ' '], "") {
        return 100;
    }
    if id_lower.contains(query_lower) {
        score += 40;
    }
    if title_lower.contains(query_lower) {
        score += 30;
    }
    if desc_lower.contains(query_lower) {
        score += 20;
    }

    for term in terms {
        if id_lower.contains(term) {
            score += 15;
        }
        if title_lower.contains(term) {
            score += 12;
        }
        if cat_lower.contains(term) {
            score += 10;
        }
        if desc_lower.contains(term) {
            score += 6;
        }
        for tag in util.tags {
            if tag.contains(term.as_str()) {
                score += 8;
            }
        }
    }

    // Intent boosts (Swedish + English)
    let intents: &[(&[&str], &[&str])] = &[
        (&["rest", "api", "endpoint", "route", "http"], &["http-route-get", "http-route-post", "http-serve", "project-api"]),
        (&["sql", "database", "databas", "tabell", "table"], &["sql-create-table", "sql-insert", "sql-select"]),
        (&["statistik", "stats", "mean", "medel"], &["science-stats", "science-linreg"]),
        (&["modul", "export", "pub", "library"], &["mod-pub-fn", "mod-pub-let", "mod-import-file"]),
        (&["klass", "class", "oop"], &["class-basic"]),
        (&["projekt", "project", "start", "starter"], &["project-main", "project-api"]),
        (&["crypto", "hash", "säkerhet", "security"], &["crypto-hash"]),
    ];

    for (keywords, util_ids) in intents {
        if keywords.iter().any(|k| query_lower.contains(k)) {
            if util_ids.contains(&util.id) {
                score += 18;
            }
        }
    }

    score
}

fn tokenize(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_lowercase())
        .collect()
}
