//! Kabootar DocAI — interaktiv dokumentationsassistent.
//!
//! ```bash
//! cargo run --bin kabootar-docai
//! kabootar-docai --ask "hur importerar jag science"
//! kabootar-docai --search "PLC timer" --limit 5
//! kabootar-docai --topics
//! ```

use kabootar::docai::{ask, search, topics};
use std::env;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    if args.iter().any(|a| a == "--topics") {
        for topic in topics() {
            println!("{topic}");
        }
        return;
    }

    if let Some(query) = flag_value(&args, "--ask") {
        println!("{}", ask(&query).text);
        return;
    }

    if let Some(query) = flag_value(&args, "--search") {
        let limit = flag_usize(&args, "--limit").unwrap_or(8);
        for hit in search(&query, limit) {
            println!(
                "[{}]\t{}\t{}\t{}",
                hit.score,
                hit.path,
                hit.heading,
                hit.excerpt.lines().next().unwrap_or("").trim()
            );
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
        "Kabootar DocAI v{version}\n\n\
         Användning:\n\
           kabootar-docai                         interaktiv REPL\n\
           kabootar-docai --ask \"fråga\"          ett svar till stdout\n\
           kabootar-docai --search \"fråga\"       sökträffar (TSV)\n\
           kabootar-docai --search \"x\" --limit 5\n\
           kabootar-docai --topics                lista dokument\n"
    );
}

fn run_repl() {
    let version = env!("CARGO_PKG_VERSION");
    println!("Kabootar DocAI v{version} — fråga om dokumentationen");
    println!("Kommandon: :quit  :topics  :search <fråga>");
    println!();

    loop {
        print!("docai> ");
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
        if input == ":topics" {
            println!("Tillgängliga dokument:");
            for topic in topics() {
                println!("  - {topic}");
            }
            continue;
        }
        if let Some(q) = input.strip_prefix(":search ") {
            for hit in search(q, 8) {
                println!(
                    "[{}] {} — {}\n    {}",
                    hit.score,
                    hit.path,
                    hit.heading,
                    hit.excerpt.lines().next().unwrap_or("").trim()
                );
            }
            continue;
        }

        let answer = ask(input);
        println!("\n{}\n", answer.text);
        if !answer.sources.is_empty() {
            println!("({} träffar i indexet)\n", answer.sources.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ask_flag() {
        let args = vec![
            "kabootar-docai".into(),
            "--ask".into(),
            "science import".into(),
        ];
        assert_eq!(flag_value(&args, "--ask").as_deref(), Some("science import"));
    }
}
