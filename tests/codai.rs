//! CodAI integration tests.

use kabootar::codai::{
    compose, progress_report, project_tree, scaffold_project, suggest, suggest_projects,
    sync_project, util, IDE_PATH, PROGRESS_PATH, ROADMAP_PATH, ROAD_NOW_PATH,
};
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::modules::import_module;
use kabootar::value::Value;
use std::fs;

#[test]
fn codai_util_returns_http_snippet() {
    let code = util("http-route-get").unwrap();
    assert!(code.contains("http_route"));
    assert!(code.contains("import \"http\""));
}

#[test]
fn codai_suggest_rest_api() {
    let hits = suggest("REST API", 5);
    assert!(!hits.is_empty());
    let ids: Vec<_> = hits.iter().map(|h| h.id.as_str()).collect();
    assert!(ids.iter().any(|id| id.starts_with("http-") || id.starts_with("project-")));
}

#[test]
fn codai_compose_dedupes_imports() {
    let code = compose(&["http-health", "http-serve"]).unwrap();
    let import_count = code.matches("import \"http\"").count();
    assert_eq!(import_count, 1);
    assert!(code.contains("http_serve"));
    assert!(code.contains("/health"));
}

#[test]
fn codai_import_registers_natives() {
    let mut env = create_global_env();
    import_module("codai", &mut env).unwrap();

    let utils = eval_source("code_utils()", &mut env).unwrap();
    assert!(matches!(utils, Value::Array(items) if items.len() >= 20));

    let snippet = eval_source("code_util(\"sql-insert\")", &mut env).unwrap();
    assert!(matches!(snippet, Value::String(s) if s.contains("INSERT INTO")));

    let composed = eval_source(
        "code_compose([\"http-health\", \"http-serve\"])",
        &mut env,
    )
    .unwrap();
    assert!(matches!(composed, Value::String(s) if s.contains("http_serve")));
}

#[test]
fn codai_unknown_util_hint() {
    let err = util("http-get-route").unwrap_err();
    assert!(err.contains("Did you mean") || err.contains("unknown utility"));
}

#[test]
fn codai_project_suggest_api() {
    let hits = suggest_projects("REST API med databas", 5);
    assert!(!hits.is_empty());
    let ids: Vec<_> = hits.iter().map(|h| h.id.as_str()).collect();
    assert!(ids.iter().any(|id| *id == "api" || *id == "api-crud"));
}

#[test]
fn codai_project_tree_lists_files() {
    let tree = project_tree("api").unwrap();
    assert!(tree.contains("kabootar.toml"));
    assert!(tree.contains("lib/routes.kab"));
}

#[test]
fn codai_sync_updates_progress_and_road() {
    let base = std::env::temp_dir().join(format!("kabootar-codai-sync-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    scaffold_project("api", &base, false).unwrap();
    assert!(base.join(PROGRESS_PATH).exists());
    assert!(base.join(ROADMAP_PATH).exists());
    assert!(base.join(IDE_PATH).exists());

    // Simulate development: add extra route
    let main = base.join("main.kab");
    let mut content = fs::read_to_string(&main).unwrap();
    content.push_str("\nhttp_route(\"GET\", \"/api/v2\", list_items)\n");
    fs::write(&main, content).unwrap();

    let report = sync_project(&base).unwrap();
    assert!(report.updated.contains(&PROGRESS_PATH.to_string()));
    assert!(report.completion_pct > 0);

    let progress = fs::read_to_string(base.join(PROGRESS_PATH)).unwrap();
    assert!(progress.contains("CodAI sync"));
    assert!(progress.contains("route"));

    let ide = fs::read_to_string(base.join(IDE_PATH)).unwrap();
    assert!(ide.contains("VS Code"));
    assert!(ide.contains("Cursor"));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn codai_sync_via_native() {
    let base = std::env::temp_dir().join(format!("kabootar-codai-sync-native-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    scaffold_project("web", &base, false).unwrap();

    let mut env = create_global_env();
    import_module("codai", &mut env).unwrap();
    let path = base.to_string_lossy().replace('\\', "/");
    let val = eval_source(&format!("code_project_sync(\"{path}\")"), &mut env).unwrap();
    assert!(matches!(val, Value::String(s) if s.contains("CodAI sync") && s.contains("road/")));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn codai_scaffold_creates_progress_txt() {
    let base = std::env::temp_dir().join(format!("kabootar-codai-progress-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let report = scaffold_project("api", &base, false).unwrap();
    assert!(report.created.iter().any(|p| p == PROGRESS_PATH));

    let progress = fs::read_to_string(base.join(PROGRESS_PATH)).unwrap();
    assert!(progress.contains("NÄSTA STEG") || progress.contains("CodAI sync"));
    assert!(progress.contains("REST API"));
    assert!(progress.contains("lib/routes.kab"));
    assert!(!progress.contains("# ")); // vanlig text, inte markdown

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn codai_progress_report_api() {
    let md = progress_report("api").unwrap();
    assert!(md.contains("VAD DU HAR ÅSTADKOMMIT"));
    assert!(md.contains("kabootar serve"));
}

#[test]
fn codai_scaffold_creates_project_files() {
    let base = std::env::temp_dir().join(format!("kabootar-codai-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let report = scaffold_project("library", &base, false).unwrap();
    assert!(report.created.iter().any(|p| p == "kabootar.toml"));
    assert!(report.created.iter().any(|p| p == "lib/greet.kab"));
    assert!(base.join("demo.kab").exists());

    // Second run skips existing code files; sync uppdaterar PROGRESS.txt och road/
    let report2 = scaffold_project("library", &base, false).unwrap();
    assert!(report2.skipped.iter().any(|p| p == "kabootar.toml"));
    assert!(report2.created.iter().any(|p| p == PROGRESS_PATH));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn codai_scaffold_via_native() {
    let base = std::env::temp_dir().join(format!("kabootar-codai-native-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let mut env = create_global_env();
    import_module("codai", &mut env).unwrap();
    let path = base.to_string_lossy().replace('\\', "/");
    let src = format!("code_project_scaffold(\"science\", \"{path}\")");
    let val = eval_source(&src, &mut env).unwrap();
    assert!(matches!(val, Value::String(s) if s.contains("Skapade") && s.contains("main.kab")));
    assert!(base.join("lib/data.kab").exists());

    let _ = fs::remove_dir_all(&base);
}
