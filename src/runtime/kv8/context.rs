//! Kv8 runtime context — isolate-like document + CSS + JS-scope state.

use crate::runtime::kabootar_dom::{assign_ids, DomNode, KabootarDocument};
use crate::runtime::kstyle::{compute_style, parse_stylesheet, ComputedStyle, Stylesheet};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum Kv8Value {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Dom(DomNode),
    Obj(HashMap<String, Kv8Value>),
    /// Interpreted `function` / block body
    Fun {
        params: Vec<String>,
        body: Vec<super::ast::Stmt>,
    },
    /// Arrow `=>` — bytecode cached on first call
    Arrow {
        params: Vec<String>,
        body: Box<super::ast::Expr>,
    },
}

impl Kv8Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Undefined | Self::Null | Self::Bool(false) => false,
            Self::Num(n) => *n != 0.0,
            Self::Str(s) => !s.is_empty(),
            Self::Obj(m) => !m.is_empty(),
            Self::Dom(_) => true,
            Self::Bool(true) => true,
            Self::Fun { .. } | Self::Arrow { .. } => true,
        }
    }

    pub fn as_obj(&self) -> Option<&HashMap<String, Kv8Value>> {
        match self {
            Self::Obj(m) => Some(m),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Kv8ContextInner {
    pub document: KabootarDocument,
    pub stylesheet: Stylesheet,
    pub css_text: String,
    pub scope: HashMap<String, Kv8Value>,
    pub last_result: Kv8Value,
    pub jit: Option<super::jit::Kv8Jit>,
    pub opt: super::opt::Kv8OptState,
}

impl Default for Kv8ContextInner {
    fn default() -> Self {
        Self {
            document: KabootarDocument::new(),
            stylesheet: Stylesheet::default(),
            css_text: String::new(),
            scope: HashMap::new(),
            last_result: Kv8Value::Undefined,
            jit: Some(super::jit::Kv8Jit::default()),
            opt: super::opt::Kv8OptState::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Kv8Context(pub Arc<Mutex<Kv8ContextInner>>);

impl Default for Kv8Context {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Kv8ContextInner::default())))
    }
}

impl Kv8Context {
    pub fn with_mut<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut Kv8ContextInner) -> Result<T, String>,
    {
        let mut g = self
            .0
            .lock()
            .map_err(|_| "kv8 context lock poisoned".to_string())?;
        f(&mut g)
    }

    fn ensure_dom_index(&self) -> Result<(), String> {
        self.with_mut(|ctx| {
            if ctx.opt.dom_index_dirty {
                super::opt::rebuild_dom_index(&ctx.document.root, &mut ctx.opt.dom_paths);
                ctx.opt.dom_index_dirty = false;
            }
            Ok(())
        })
    }

    fn mark_dom_dirty(&self) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.opt.dom_index_dirty = true;
            Ok(())
        })
    }

    pub fn set_stylesheet(&self, css: &str) -> Result<usize, String> {
        self.with_mut(|ctx| {
            if ctx.css_text == css {
                return Ok(ctx.stylesheet.rules.len());
            }
            ctx.css_text = css.to_string();
            ctx.stylesheet = parse_stylesheet(css);
            ctx.opt.style_generation = ctx.opt.style_generation.wrapping_add(1);
            ctx.opt.style_cache.clear();
            Ok(ctx.stylesheet.rules.len())
        })
    }

    pub fn computed_style(&self, node: &DomNode) -> Result<ComputedStyle, String> {
        self.with_mut(|ctx| {
            if let Some(cached) = ctx.opt.style_cache.get(&node.id) {
                return Ok(cached.clone());
            }
            let style = compute_style(&node.tag, &node.attributes, &ctx.stylesheet);
            ctx.opt.style_cache.insert(node.id, style.clone());
            Ok(style)
        })
    }

    pub fn root_dom(&self) -> Result<DomNode, String> {
        self.with_mut(|ctx| Ok(ctx.document.root.clone()))
    }

    pub fn create_element(&self, tag: &str) -> Result<DomNode, String> {
        self.with_mut(|ctx| {
            let mut el = DomNode::element(tag);
            assign_ids(&mut el);
            Ok(el)
        })
    }

    pub fn append_child(&self, parent_id: u64, child: DomNode) -> Result<bool, String> {
        let ok = self.with_mut(|ctx| {
            if let Some(p) = find_mut_by_id(&mut ctx.document.root, parent_id) {
                p.append(child);
                return Ok(true);
            }
            if ctx.document.root.id == parent_id {
                ctx.document.root.append(child);
                return Ok(true);
            }
            Ok(false)
        })?;
        if ok {
            self.mark_dom_dirty()?;
        }
        Ok(ok)
    }

    pub fn query_selector(&self, selector: &str) -> Result<Option<DomNode>, String> {
        self.ensure_dom_index()?;
        self.with_mut(|ctx| Ok(find_by_selector(&ctx.document.root, selector).cloned()))
    }

    pub fn set_text_content(&self, id: u64, text: &str) -> Result<bool, String> {
        self.ensure_dom_index()?;
        let ok = self.mutate_node(id, |n| {
            n.text = Some(text.to_string());
        })?;
        if ok {
            self.with_mut(|ctx| {
                ctx.opt.style_generation = ctx.opt.style_generation.wrapping_add(1);
                ctx.opt.style_cache.remove(&id);
                Ok(())
            })?;
        }
        Ok(ok)
    }

    pub fn set_attr(&self, id: u64, key: &str, value: &str) -> Result<bool, String> {
        self.ensure_dom_index()?;
        let ok = self.mutate_node(id, |n| {
            n.set_attr(key, value);
        })?;
        if ok {
            self.with_mut(|ctx| {
                ctx.opt.style_generation = ctx.opt.style_generation.wrapping_add(1);
                ctx.opt.style_cache.remove(&id);
                Ok(())
            })?;
        }
        Ok(ok)
    }

    fn mutate_node<F>(&self, id: u64, f: F) -> Result<bool, String>
    where
        F: FnOnce(&mut DomNode),
    {
        self.with_mut(|ctx| {
            if let Some(path) = ctx.opt.dom_paths.get(&id).cloned() {
                if let Some(n) = super::opt::find_mut_by_path(&mut ctx.document.root, &path) {
                    f(n);
                    return Ok(true);
                }
            }
            if let Some(n) = find_mut_by_id(&mut ctx.document.root, id) {
                f(n);
                return Ok(true);
            }
            Ok(false)
        })
    }
}

fn find_mut_by_id<'a>(node: &'a mut DomNode, id: u64) -> Option<&'a mut DomNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(found) = find_mut_by_id(child, id) {
            return Some(found);
        }
    }
    None
}

fn find_by_selector<'a>(node: &'a DomNode, selector: &str) -> Option<&'a DomNode> {
    let sel = selector.trim();
    if sel.starts_with('#') {
        let id_attr = &sel[1..];
        return find_by_attr(node, "id", id_attr);
    }
    if sel.starts_with('.') {
        let cls = &sel[1..];
        return find_by_attr(node, "class", cls);
    }
    node.query_tag(sel)
}

fn find_by_attr<'a>(node: &'a DomNode, key: &str, value: &str) -> Option<&'a DomNode> {
    if node.get_attr(key) == Some(value) {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_by_attr(child, key, value) {
            return Some(found);
        }
    }
    None
}
