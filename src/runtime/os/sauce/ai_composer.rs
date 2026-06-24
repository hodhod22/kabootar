//! Strategy 1 — Invisible AI composer: predictive prefetch + contextual UI.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HabitEntry {
    pub app: String,
    pub launches: u64,
    pub hour_bucket: u8,
}

pub struct AiComposer {
    habits: HashMap<String, HabitEntry>,
    prefetch_queue: Vec<String>,
    drafts: HashMap<String, String>,
}

impl Default for AiComposer {
    fn default() -> Self {
        let mut s = Self {
            habits: HashMap::new(),
            prefetch_queue: Vec::new(),
            drafts: HashMap::new(),
        };
        for app in ["spotify", "mail", "teams"] {
            s.record_launch(app, 8);
        }
        s.rebuild_prefetch();
        s.drafts.insert(
            "mail:compose".into(),
            "Hej — här är ett AI-utkast baserat på dina vanor.".into(),
        );
        s
    }
}

impl AiComposer {
    pub fn record_launch(&mut self, app: &str, hour: u8) {
        let e = self
            .habits
            .entry(app.to_string())
            .or_insert(HabitEntry {
                app: app.to_string(),
                launches: 0,
                hour_bucket: hour,
            });
        e.launches += 1;
        e.hour_bucket = hour;
        self.rebuild_prefetch();
    }

    fn rebuild_prefetch(&mut self) {
        let mut ranked: Vec<_> = self.habits.values().cloned().collect();
        ranked.sort_by(|a, b| b.launches.cmp(&a.launches));
        self.prefetch_queue = ranked.into_iter().take(5).map(|h| h.app).collect();
    }

    pub fn prefetch_targets(&self) -> &[String] {
        &self.prefetch_queue
    }

    pub fn contextual_menu(&self, app: &str, items: &[String]) -> Vec<String> {
        let top = self
            .habits
            .get(app)
            .map(|h| h.launches)
            .unwrap_or(0);
        let keep = if top > 3 { 4 } else { 6 };
        items.iter().take(keep).cloned().collect()
    }

    pub fn debug_suggest(&self, context: &str) -> Option<String> {
        let key = format!("debug:{context}");
        if let Some(s) = self.drafts.get(&key) {
            return Some(s.clone());
        }
        if context.contains("breakpoint") || context.contains("crash") {
            return Some(
                "Kabootar AI: kontrollera null-referens och scope i Kv8-script; föreslår devtools_breakpoint."
                    .into(),
            );
        }
        self.drafts.get(context).cloned()
    }

    pub fn set_draft(&mut self, context: &str, text: &str) {
        self.drafts.insert(context.to_string(), text.to_string());
    }
}
