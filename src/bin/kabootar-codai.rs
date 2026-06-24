//! Kabootar CodAI — utility-first kodassistent (Tailwind för Kabootar).
//!
//! ```bash
//! cargo run --bin kabootar-codai
//! kabootar-codai --util http-route-get
//! kabootar-codai --suggest "REST API"
//! kabootar-codai --compose http-health http-serve
//! ```

use kabootar::codai::{
    all_ids, all_project_ids, categories, compose, help, project_tree, scaffold_project,
    suggest, suggest_projects, sync_project, util,
};
use std::env;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    if args.iter().any(|a| a == "--utils") {
        for id in all_ids() {
            println!("{id}");
        }
        return;
    }

    if args.iter().any(|a| a == "--categories") {
        for cat in categories() {
            println!("{cat}");
        }
        return;
    }

    if args.iter().any(|a| a == "--projects") {
        for id in all_project_ids() {
            println!("{id}");
        }
        return;
    }

    if let Some(query) = flag_value(&args, "--project-suggest") {
        let limit = flag_usize(&args, "--limit").unwrap_or(5);
        for hit in suggest_projects(&query, limit) {
            println!(
                "[{}]\t{}\t{}\t{}",
                hit.score, hit.id, hit.title, hit.description
            );
        }
        return;
    }

    if let Some(id) = flag_value(&args, "--project-tree") {
        match project_tree(&id) {
            Ok(tree) => print!("{tree}"),
            Err(e) => eprintln!("{e}"),
        }
        return;
    }

    if let Some(id) = flag_value(&args, "--project-progress") {
        match kabootar::codai::progress_report(&id) {
            Ok(md) => print!("{md}"),
            Err(e) => eprintln!("{e}"),
        }
        return;
    }

    if args.iter().any(|a| a == "--project-sync") {
        let path = flag_value(&args, "--project-sync").unwrap_or_else(|| ".".into());
        match sync_project(std::path::Path::new(&path)) {
            Ok(report) => {
                use kabootar::codai::format_sync_report;
                print!("{}", format_sync_report(&report));
            }
            Err(e) => eprintln!("{e}"),
        }
        return;
    }

    if let Some(id) = flag_value(&args, "--project-scaffold") {
        let dir = flag_value(&args, "--dir").unwrap_or_else(|| ".".into());
        let force = args.iter().any(|a| a == "--force");
        match scaffold_project(&id, std::path::Path::new(&dir), force) {
            Ok(report) => {
                use kabootar::codai::format_scaffold_report;
                print!("{}", format_scaffold_report(&id, &report));
            }
            Err(e) => eprintln!("{e}"),
        }
        return;
    }

    if let Some(id) = flag_value(&args, "--util") {
        match util(&id) {
            Ok(code) => print!("{code}"),
            Err(e) => eprintln!("{e}"),
        }
        return;
    }

    if let Some(query) = flag_value(&args, "--suggest") {
        let limit = flag_usize(&args, "--limit").unwrap_or(8);
        for hit in suggest(&query, limit) {
            println!(
                "[{}]\t{}\t{}\t{}",
                hit.score, hit.id, hit.title, hit.description
            );
        }
        return;
    }

    if let Some(topic) = flag_value(&args, "--help-topic") {
        println!("{}", help(&topic));
        return;
    }

    if let Some(ids_str) = flag_value(&args, "--compose") {
        let ids: Vec<&str> = ids_str.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
        match compose(&ids) {
            Ok(code) => print!("{code}"),
            Err(e) => eprintln!("{e}"),
        }
        return;
    }

    run_repl();
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn flag_usize(args: &[String], name: &str) -> Option<usize> {
    flag_value(args, name)?.parse().ok()
}

fn print_help() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "Kabootar CodAI v{version}\n\n\
         Utility-first kodbyggblock — som Tailwind för CSS.\n\n\
         Användning:\n\
           kabootar-codai                              interaktiv REPL\n\
           kabootar-codai --utils                      lista alla utilities\n\
           kabootar-codai --categories                 lista kategorier\n\
           kabootar-codai --projects                   lista projektmallar\n\
           kabootar-codai --project-suggest \"REST API\"  föreslå projektstruktur\n\
           kabootar-codai --project-tree api           visa mappträd\n\
           kabootar-codai --project-scaffold api --dir ./myapp\n\
           kabootar-codai --project-sync --dir .\n\
           kabootar-codai --util http-route-get        skriv ut kodmall\n\
           kabootar-codai --suggest \"REST API\"       föreslå utilities\n\
           kabootar-codai --compose id1,id2            slå ihop utilities\n\
           kabootar-codai --help-topic http            hjälp per kategori\n"
    );
}

fn run_repl() {
    let version = env!("CARGO_PKG_VERSION");
    println!("Kabootar CodAI v{version} — utility-first kodassistent");
    println!("Kommandon: :quit  :utils  :projects  :project-sync [dir]  :project-suggest <fråga>  :project-tree <id>  :project-scaffold <id>[,dir]  :util <id>  :suggest <fråga>  :compose id1,id2");
    println!();

    loop {
        print!("codai> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == ":quit" || input == ":exit" {
            break;
        }
        if input == ":utils" {
            for id in all_ids() {
                println!("  {id}");
            }
            continue;
        }
        if input == ":projects" {
            for id in all_project_ids() {
                println!("  {id}");
            }
            continue;
        }
        if let Some(dir) = input.strip_prefix(":project-sync") {
            let path = dir.trim();
            let path = if path.is_empty() { "." } else { path };
            match sync_project(std::path::Path::new(path)) {
                Ok(report) => {
                    use kabootar::codai::format_sync_report;
                    println!("{}", format_sync_report(&report));
                }
                Err(e) => println!("{e}"),
            }
            continue;
        }
        if let Some(q) = input.strip_prefix(":project-suggest ") {
            for hit in suggest_projects(q.trim(), 5) {
                println!("[{}] {} — {}", hit.score, hit.id, hit.title);
            }
            continue;
        }
        if let Some(id) = input.strip_prefix(":project-tree ") {
            match project_tree(id.trim()) {
                Ok(tree) => println!("\n{tree}"),
                Err(e) => println!("{e}"),
            }
            continue;
        }
        if let Some(rest) = input.strip_prefix(":project-scaffold ") {
            let parts: Vec<&str> = rest.splitn(2, ',').map(str::trim).collect();
            let id = parts.first().copied().unwrap_or("");
            let dir = parts.get(1).copied().unwrap_or(".");
            match scaffold_project(id, std::path::Path::new(dir), false) {
                Ok(report) => {
                    use kabootar::codai::format_scaffold_report;
                    println!("{}", format_scaffold_report(id, &report));
                }
                Err(e) => println!("{e}"),
            }
            continue;
        }
        if input == ":categories" {
            for cat in categories() {
                println!("  {cat}");
            }
            continue;
        }
        if let Some(id) = input.strip_prefix(":util ") {
            match util(id.trim()) {
                Ok(code) => println!("\n{code}"),
                Err(e) => println!("{e}"),
            }
            continue;
        }
        if let Some(q) = input.strip_prefix(":suggest ") {
            for hit in suggest(q.trim(), 8) {
                println!(
                    "[{}] {} — {}",
                    hit.score, hit.id, hit.title
                );
            }
            continue;
        }
        if let Some(ids) = input.strip_prefix(":compose ") {
            let parts: Vec<&str> = ids.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
            match compose(&parts) {
                Ok(code) => println!("\n{code}"),
                Err(e) => println!("{e}"),
            }
            continue;
        }

        println!("{}", help(input));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_util_flag() {
        let args = vec![
            "kabootar-codai".into(),
            "--util".into(),
            "http-route-get".into(),
        ];
        assert_eq!(flag_value(&args, "--util").as_deref(), Some("http-route-get"));
    }
}
