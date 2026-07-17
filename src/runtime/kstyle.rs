//! Kabootar Style Sheet (KSS) — minimal CSS engine for Kabootar DOM.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ComputedStyle {
    pub display: String,
    pub color: String,
    pub background: String,
    pub font_size: String,
    pub font_weight: String,
    pub padding: String,
    pub margin: String,
    pub width: String,
    pub height: String,
    pub border_radius: String,
    pub flex_direction: String,
    pub justify_content: String,
    pub align_items: String,
    pub flex_wrap: String,
    pub flex_grow: f64,
    pub flex_shrink: f64,
    pub gap: String,
    pub line_height: String,
    pub white_space: String,
    pub grid_template_columns: String,
}

impl ComputedStyle {
    pub fn base() -> Self {
        Self {
            display: "block".into(),
            color: "#e8eaed".into(),
            background: "transparent".into(),
            font_size: "16px".into(),
            font_weight: "400".into(),
            padding: "0".into(),
            margin: "0".into(),
            width: "auto".into(),
            height: "auto".into(),
            border_radius: "0".into(),
            flex_direction: "column".into(),
            justify_content: "flex-start".into(),
            align_items: "stretch".into(),
            flex_wrap: "nowrap".into(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            gap: "0".into(),
            line_height: "normal".into(),
            white_space: "normal".into(),
            grid_template_columns: String::new(),
        }
    }

    pub fn to_inline_css(&self) -> String {
        format!(
            "display:{};color:{};background:{};font-size:{};font-weight:{};\
             padding:{};margin:{};width:{};height:{};border-radius:{};\
             flex-direction:{};justify-content:{};align-items:{};flex-wrap:{};\
             flex-grow:{};flex-shrink:{};gap:{};\
             grid-template-columns:{};line-height:{};white-space:{};box-sizing:border-box;",
            self.display,
            self.color,
            self.background,
            self.font_size,
            self.font_weight,
            self.padding,
            self.margin,
            self.width,
            self.height,
            self.border_radius,
            self.flex_direction,
            self.justify_content,
            self.align_items,
            self.flex_wrap,
            self.flex_grow,
            self.flex_shrink,
            self.gap,
            self.grid_template_columns,
            self.line_height,
            self.white_space,
        )
    }

    pub fn apply_decl(&mut self, prop: &str, value: &str) {
        let v = value.trim();
        match prop.trim().to_ascii_lowercase().as_str() {
            "display" => self.display = v.into(),
            "color" => self.color = v.into(),
            "background" | "background-color" => self.background = v.into(),
            "font-size" => self.font_size = v.into(),
            "font-weight" => self.font_weight = v.into(),
            "padding" => self.padding = v.into(),
            "margin" => self.margin = v.into(),
            "width" => self.width = v.into(),
            "height" => self.height = v.into(),
            "border-radius" => self.border_radius = v.into(),
            "flex-direction" => self.flex_direction = v.into(),
            "justify-content" => self.justify_content = v.into(),
            "align-items" => self.align_items = v.into(),
            "flex-wrap" => self.flex_wrap = v.into(),
            "flex-grow" => self.flex_grow = v.parse().unwrap_or(0.0),
            "flex-shrink" => self.flex_shrink = v.parse().unwrap_or(1.0),
            "flex" => apply_flex_shorthand(self, v),
            "gap" => self.gap = v.into(),
            "grid-template-columns" => self.grid_template_columns = v.into(),
            "line-height" => self.line_height = v.into(),
            "white-space" => self.white_space = v.into(),
            _ => {}
        }
    }
}

fn apply_flex_shorthand(style: &mut ComputedStyle, v: &str) {
    let parts: Vec<&str> = v.split_whitespace().collect();
    match parts.as_slice() {
        ["none"] => {
            style.flex_grow = 0.0;
            style.flex_shrink = 0.0;
        }
        ["auto"] => {
            style.flex_grow = 1.0;
            style.flex_shrink = 1.0;
        }
        [grow] => {
            if let Ok(g) = grow.parse::<f64>() {
                style.flex_grow = g;
                style.flex_shrink = 1.0;
            }
        }
        [grow, shrink, ..] => {
            if let Ok(g) = grow.parse::<f64>() {
                style.flex_grow = g;
            }
            if let Ok(s) = shrink.parse::<f64>() {
                style.flex_shrink = s;
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,
    pub declarations: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<StyleRule>,
}

pub fn parse_stylesheet(input: &str) -> Stylesheet {
    let mut sheet = Stylesheet::default();
    let mut i = 0usize;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        while i < bytes.len() && bytes[i] != b'{' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let selector = String::from_utf8_lossy(&bytes[start..i]).trim().to_string();
        i += 1;
        let decl_start = i;
        while i < bytes.len() && bytes[i] != b'}' {
            i += 1;
        }
        let decls = String::from_utf8_lossy(&bytes[decl_start..i]).to_string();
        i += 1;
        if !selector.is_empty() {
            sheet.rules.push(StyleRule {
                selector,
                declarations: parse_declarations(&decls),
            });
        }
    }
    sheet
}

fn parse_declarations(input: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for part in input.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((k, v)) = part.split_once(':') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

pub fn compute_style(
    tag: &str,
    attrs: &HashMap<String, String>,
    sheet: &Stylesheet,
) -> ComputedStyle {
    let mut style = ComputedStyle::base();
    for rule in &sheet.rules {
        if selector_matches(&rule.selector, tag, attrs) {
            for (k, v) in &rule.declarations {
                style.apply_decl(k, v);
            }
        }
    }
    if let Some(inline) = attrs.get("style") {
        for (k, v) in parse_declarations(inline) {
            style.apply_decl(&k, &v);
        }
    }
    for (k, v) in attrs {
        if let Some(prop) = k.strip_prefix("style:") {
            style.apply_decl(prop, v);
        }
    }
    style
}

fn selector_matches(selector: &str, tag: &str, attrs: &HashMap<String, String>) -> bool {
    let sel = selector.trim();
    if sel == tag {
        return true;
    }
    if let Some(class) = sel.strip_prefix('.') {
        return attrs
            .get("class")
            .map(|c| c.split_whitespace().any(|x| x == class))
            .unwrap_or(false);
    }
    if let Some(id) = sel.strip_prefix('#') {
        return attrs.get("id").map(|v| v == id).unwrap_or(false);
    }
    false
}
