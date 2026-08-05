//! `kabootar registry web` — browse local package registry over HTTP.

use crate::registry::{self, PackageInfo};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;

pub fn render_index(packages: &[PackageInfo]) -> String {
    let mut rows = String::new();
    for p in packages {
        rows.push_str(&format!(
            "<tr><td><code>{}</code></td><td><code>{}</code></td></tr>\n",
            html_escape(&p.name),
            html_escape(&p.version)
        ));
    }
    if rows.is_empty() {
        rows.push_str("<tr><td colspan=\"2\"><em>No packages in local registry</em></td></tr>\n");
    }
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<title>Kabootar Registry</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; background: #0f172a; color: #e2e8f0; }}
h1 {{ font-weight: 600; }}
a {{ color: #38bdf8; }}
table {{ border-collapse: collapse; width: 100%; max-width: 640px; }}
th, td {{ border-bottom: 1px solid #334155; padding: 0.5rem 0.75rem; text-align: left; }}
th {{ color: #94a3b8; font-size: 0.85rem; }}
code {{ font-family: ui-monospace, monospace; }}
.muted {{ color: #94a3b8; font-size: 0.9rem; }}
</style>
</head>
<body>
<h1>Kabootar local registry</h1>
<p class="muted">Packages under <code>.kabootar/registry/</code> — publish with <code>kabootar publish</code>.</p>
<table>
<thead><tr><th>Name</th><th>Version</th></tr></thead>
<tbody>
{rows}
</tbody>
</table>
<p class="muted"><a href="/api/packages">JSON API</a></p>
</body>
</html>
"#
    )
}

pub fn render_json(packages: &[PackageInfo]) -> String {
    let mut items = Vec::new();
    for p in packages {
        items.push(format!(
            "{{\"name\":{},\"version\":{}}}",
            json_str(&p.name),
            json_str(&p.version)
        ));
    }
    format!("{{\"packages\":[{}]}}", items.join(","))
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn serve_registry_web(base: &Path, bind: &str, port: u16) -> Result<(), String> {
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr).map_err(|e| format!("bind {addr}: {e}"))?;
    eprintln!("Registry web: http://{addr}/");
    for stream in listener.incoming() {
        let mut stream = stream.map_err(|e| format!("accept: {e}"))?;
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.lines().next().unwrap_or("");
        let packages = registry::list_registry(base).unwrap_or_default();
        let (status, content_type, body) = if path.contains(" /api/packages") {
            ("200 OK", "application/json; charset=utf-8", render_json(&packages))
        } else if path.contains(" / ") || path.contains(" /index") || path.starts_with("GET / HTTP")
        {
            ("200 OK", "text/html; charset=utf-8", render_index(&packages))
        } else {
            (
                "404 Not Found",
                "text/plain; charset=utf-8",
                "not found".into(),
            )
        };
        let resp = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(resp.as_bytes());
    }
    Ok(())
}
