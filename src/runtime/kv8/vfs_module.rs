//! `.kv8` VFS modules — KML + CSS + script bundles for browser + Kv8.

use super::context::Kv8Context;
use super::eval::eval_script;
use crate::kml::parse_kml;
use crate::runtime::kabootar_dom::{assign_ids, DomNode};
use crate::runtime::kstyle::parse_stylesheet;
use crate::runtime::os::OsHandle;

#[derive(Debug, Clone)]
pub struct Kv8Module {
    pub kml: String,
    pub css: String,
    pub script: String,
}

pub fn parse_kv8_module(source: &str) -> Result<Kv8Module, String> {
    let kml_key = "---kml---";
    let css_key = "---css---";
    let script_key = "---script---";
    if source.contains(kml_key) {
        let after_kml = source.split_once(kml_key).map(|(_, r)| r).unwrap_or(source);
        let (kml_part, rest) = split_at_marker(after_kml, css_key);
        let (css_part, script_part) = split_at_marker(&rest, script_key);
        return Ok(Kv8Module {
            kml: kml_part.trim().to_string(),
            css: css_part.trim().to_string(),
            script: script_part.trim().to_string(),
        });
    }
    if source.trim_start().starts_with('<') {
        return Ok(Kv8Module {
            kml: source.to_string(),
            css: String::new(),
            script: String::new(),
        });
    }
    Ok(Kv8Module {
        kml: "<html><body></body></html>".into(),
        css: String::new(),
        script: source.to_string(),
    })
}

fn split_at_marker(input: &str, marker: &str) -> (String, String) {
    if let Some((a, b)) = input.split_once(marker) {
        (a.to_string(), b.to_string())
    } else {
        (input.to_string(), String::new())
    }
}

pub fn kml_from_module(module: &Kv8Module) -> Result<DomNode, String> {
    let mut node = parse_kml(&module.kml)?;
    assign_ids(&mut node);
    Ok(node)
}

pub fn apply_module(ctx: &Kv8Context, module: &Kv8Module) -> Result<DomNode, String> {
    let mut root = kml_from_module(module)?;
    ctx.with_mut(|inner| {
        inner.document.root = root.clone();
        inner.css_text = module.css.clone();
        inner.stylesheet = parse_stylesheet(&module.css);
        Ok(())
    })?;
    if !module.script.is_empty() {
        eval_script(ctx, &module.script)?;
        root = ctx.root_dom()?;
    }
    Ok(root)
}

pub fn load_vfs_module(ctx: &Kv8Context, os: &OsHandle, path: &str) -> Result<DomNode, String> {
    let content = os.read(path)?;
    let module = parse_kv8_module(&content)?;
    apply_module(ctx, &module)
}
