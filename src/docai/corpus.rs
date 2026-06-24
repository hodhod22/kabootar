//! Embedded documentation corpus (compile-time `include_str!`).

#[derive(Debug, Clone)]
pub struct DocChunk {
    pub path: &'static str,
    pub title: String,
    pub heading: String,
    pub excerpt: String,
    pub body_lower: String,
}

const FILES: &[(&str, &str)] = &[
    ("README.md", include_str!("../../docs/README.md")),
    ("OVERVIEW.md", include_str!("../../docs/OVERVIEW.md")),
    ("LANGUAGE.md", include_str!("../../docs/LANGUAGE.md")),
    ("FEATURES.md", include_str!("../../docs/FEATURES.md")),
    ("TYPES.md", include_str!("../../docs/TYPES.md")),
    ("CLASSES.md", include_str!("../../docs/CLASSES.md")),
    ("KML.md", include_str!("../../docs/KML.md")),
    ("OS.md", include_str!("../../docs/OS.md")),
    ("MODULES.md", include_str!("../../docs/MODULES.md")),
    ("HTTP.md", include_str!("../../docs/HTTP.md")),
    ("SQL.md", include_str!("../../docs/SQL.md")),
    ("LSP.md", include_str!("../../docs/LSP.md")),
    ("SECURITY.md", include_str!("../../docs/SECURITY.md")),
    ("SCIENCE.md", include_str!("../../docs/SCIENCE.md")),
    ("RUNTIME.md", include_str!("../../docs/RUNTIME.md")),
    ("JAVASCRIPT.md", include_str!("../../docs/JAVASCRIPT.md")),
    ("IDE.md", include_str!("../../docs/IDE.md")),
    ("CODAI.md", include_str!("../../docs/CODAI.md")),
    ("PROJECT.md", include_str!("../../docs/PROJECT.md")),
    ("ROADMAP.md", include_str!("../../docs/ROADMAP.md")),
    ("DOCAI.md", include_str!("../../docs/DOCAI.md")),
];

pub fn topics() -> Vec<&'static str> {
    FILES
        .iter()
        .map(|(name, _)| name.trim_end_matches(".md"))
        .collect()
}

pub fn chunks() -> Vec<DocChunk> {
    let mut all = Vec::new();
    for (path, content) in FILES {
        let title = doc_title(content, path);
        all.extend(split_by_headings(path, &title, content));
    }
    all
}

fn doc_title(content: &str, path: &str) -> String {
    content
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").trim().to_string())
        .unwrap_or_else(|| path.trim_end_matches(".md").to_string())
}

fn split_by_headings(path: &'static str, title: &str, content: &str) -> Vec<DocChunk> {
    let mut chunks = Vec::new();
    let mut current_heading = title.to_string();
    let mut current_body = String::new();

    let flush = |chunks: &mut Vec<DocChunk>,
                 heading: &str,
                 body: &str,
                 path: &'static str,
                 title: &str| {
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return;
        }
        chunks.push(DocChunk {
            path,
            title: title.to_string(),
            heading: heading.to_string(),
            excerpt: trimmed.to_string(),
            body_lower: trimmed.to_lowercase(),
        });
    };

    for line in content.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            flush(&mut chunks, &current_heading, &current_body, path, title);
            current_heading = h.trim().to_string();
            current_body.clear();
        } else if let Some(h) = line.strip_prefix("### ") {
            flush(&mut chunks, &current_heading, &current_body, path, title);
            current_heading = h.trim().to_string();
            current_body.clear();
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    flush(
        &mut chunks,
        &current_heading,
        &current_body,
        path,
        title,
    );
    chunks
}
