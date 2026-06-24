//! Shell — CLI command interpreter.

pub struct Shell {
    history: Vec<String>,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            history: Vec::new(),
        }
    }
}

impl Shell {
    pub fn exec(&mut self, line: &str) -> String {
        self.history.push(line.to_string());
        let parts: Vec<_> = line.split_whitespace().collect();
        match parts.first().map(|s| *s) {
            Some("echo") => parts[1..].join(" "),
            Some("pwd") => "/".into(),
            Some("help") => "kabootar shell: echo, pwd, help, os_info".into(),
            Some("os_info") => "kabootar-kernel".into(),
            _ => format!("shell: unknown command: {line}"),
        }
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }
}
