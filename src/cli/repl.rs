//! Interactive REPL (Våg DX0 + DX5 readline/history).

use crate::session::{needs_more_input, strip_continuations, Session};
use crate::value::{format_value, Value};
use std::path::PathBuf;

const HELP: &str = "\
Kabootar REPL — exploration session (persistent env)

Commands:
  :help              This help
  :quit / :exit      Leave REPL
  :reset             Fresh environment
  :load <file.kab>   Eval file into session
  :vars              List bindings
  :science           import \"science\"
  :type <name>       Show typeof / rough kind for binding
  :history           Show recent history (readline)

Tips:
  Multiline: keep braces open, or end a line with \\
  Last result is bound to _
  Arrow-up recalls history (saved under ~/.kabootar_history)
";

fn history_path() -> PathBuf {
    dirs_next_home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".kabootar_history")
}

fn dirs_next_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn run_repl() -> i32 {
    println!(
        "Kabootar v{} — exploration REPL (type :help)",
        env!("CARGO_PKG_VERSION")
    );

    #[cfg(not(target_arch = "wasm32"))]
    {
        return run_repl_rustyline();
    }
    #[cfg(target_arch = "wasm32")]
    {
        eprintln!("REPL not available on wasm32");
        1
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_repl_rustyline() -> i32 {
    use rustyline::error::ReadlineError;
    use rustyline::DefaultEditor;

    let mut rl = match DefaultEditor::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("readline init failed: {e}");
            return 1;
        }
    };
    let hist = history_path();
    let _ = rl.load_history(&hist);

    let mut session = Session::new();
    let mut buffer = String::new();

    loop {
        let prompt = if buffer.is_empty() { "> " } else { "... " };
        let line = match rl.readline(prompt) {
            Ok(l) => l,
            Err(ReadlineError::Interrupted) => {
                buffer.clear();
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
        };
        let line_trim = line.trim_end_matches(['\r', '\n']);

        if buffer.is_empty() {
            let cmd = line_trim.trim();
            if cmd.is_empty() || cmd.starts_with("=>") {
                continue;
            }
            if cmd.starts_with(':') {
                let _ = rl.add_history_entry(cmd);
                if handle_command(cmd, &mut session, &rl) {
                    break;
                }
                continue;
            }
        }

        if !buffer.is_empty() {
            buffer.push('\n');
        }
        buffer.push_str(line_trim);

        if needs_more_input(&buffer) {
            continue;
        }

        let source = strip_continuations(&buffer);
        buffer.clear();
        let _ = rl.add_history_entry(source.as_str());

        match session.eval_cell(&source) {
            Ok(v) => {
                if !matches!(v, Value::Undefined | Value::Null) {
                    let shown = if let Some(Value::NativeFunction(f)) = session.env.get("pretty") {
                        match f(&[v.clone()], &mut session.env) {
                            Ok(Value::String(s)) => s,
                            _ => format_value(&v),
                        }
                    } else {
                        format_value(&v)
                    };
                    println!("=> {shown}");
                }
            }
            Err(e) => println!("Error: {e}"),
        }
    }

    let _ = rl.save_history(&hist);
    0
}

/// Returns true if REPL should exit.
#[cfg(not(target_arch = "wasm32"))]
fn handle_command(cmd: &str, session: &mut Session, rl: &rustyline::DefaultEditor) -> bool {
    let parts: Vec<&str> = cmd.splitn(2, char::is_whitespace).collect();
    let head = parts[0];
    let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match head {
        ":quit" | ":exit" | ":q" => return true,
        ":help" | ":h" | ":?" => {
            println!("{HELP}");
            if let Ok(Value::String(extra)) =
                session.eval_cell("import \"dx/session\"; helpText()")
            {
                println!("{extra}");
            }
        }
        ":reset" => {
            session.reset();
            println!("(reset)");
        }
        ":science" => match session.import_science() {
            Ok(()) => println!("(imported science)"),
            Err(e) => println!("Error: {e}"),
        },
        ":history" => {
            let entries: Vec<_> = rl.history().iter().collect();
            let start = entries.len().saturating_sub(32);
            for (i, e) in entries.iter().enumerate().skip(start) {
                println!("{i}: {e}");
            }
        }
        ":vars" => {
            let names = session.var_names();
            if names.is_empty() {
                println!("(no bindings)");
            } else {
                for n in names {
                    println!("  {n}");
                }
            }
        }
        ":load" => {
            if arg.is_empty() {
                println!("Usage: :load <file.kab>");
            } else {
                match session.load_file(arg) {
                    Ok(v) => println!("=> {}", format_value(&v)),
                    Err(e) => println!("Error: {e}"),
                }
            }
        }
        ":type" => {
            if arg.is_empty() {
                println!("Usage: :type <name>");
            } else {
                match session.env.get(arg) {
                    Some(v) => println!("{} : {}", arg, value_kind(&v)),
                    None => println!("Error: unbound `{arg}`"),
                }
            }
        }
        other => println!("Unknown command {other} (try :help)"),
    }
    false
}

fn value_kind(v: &Value) -> &'static str {
    match v {
        Value::Undefined => "undefined",
        Value::Null => "null",
        Value::Number(_) => "number",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Bool(_) => "bool",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Function { .. }
        | Value::BytecodeFn(_)
        | Value::NativeFunction(_)
        | Value::BoundMethod(_, _)
        | Value::BoundNative(_, _) => "function",
        Value::ClassInstance(_) => "class",
        Value::BigInt(_) => "bigint",
        _ => "value",
    }
}
