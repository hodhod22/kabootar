//! Flexbox-inspired layout for Kabootar DOM.

use crate::runtime::kabootar_dom::DomNode;
use crate::runtime::kstyle::{compute_style, ComputedStyle, Stylesheet};
use crate::runtime::render::text::{layout_text, TextLayoutResult, TextStyle};

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub node_id: u64,
    pub tag: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub children: Vec<LayoutBox>,
    pub text_layout: Option<TextLayoutResult>,
}

pub struct LayoutEngine;

impl LayoutEngine {
    pub fn layout(root: &DomNode, sheet: &Stylesheet, viewport_w: f64) -> LayoutBox {
        let root_style = compute_style(&root.tag, &root.attributes, sheet);
        Self::layout_node(root, sheet, 0.0, 0.0, viewport_w, &root_style)
    }

    fn layout_node(
        node: &DomNode,
        sheet: &Stylesheet,
        x: f64,
        y: f64,
        avail_w: f64,
        inherited: &ComputedStyle,
    ) -> LayoutBox {
        if node.tag == "#text" {
            let text = node.text.as_deref().unwrap_or("");
            let text_style = TextStyle::from_computed(inherited, Some(avail_w as f32));
            let tl = layout_text(text, &text_style);
            return LayoutBox {
                node_id: node.id,
                tag: node.tag.clone(),
                x,
                y,
                w: tl.width.max(1.0) as f64,
                h: tl.height.max(1.0) as f64,
                children: vec![],
                text_layout: Some(tl),
            };
        }

        let style = compute_style(&node.tag, &node.attributes, sheet);
        let pad = parse_px(&style.padding, 8.0);
        let margin = parse_px(&style.margin, 0.0);
        let inner_w = if node.tag == "canvas" {
            node.attributes
                .get("width")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(300.0)
        } else if style.width == "auto" {
            avail_w - margin * 2.0
        } else {
            parse_px(&style.width, avail_w)
        };

        let mut cy = y + margin + pad;
        let mut max_child_w = 0.0f64;
        let mut total_h = pad * 2.0 + margin * 2.0;
        let mut children_layout = Vec::new();

        let is_flex = style.display == "flex" || node.tag == "body" || node.tag == "div";
        let gap = parse_px(&style.gap, 8.0);

        if node.tag != "canvas" {
            if is_flex {
                let mut cx = x + margin + pad;
                for child in &node.children {
                    let child_box = Self::layout_node(child, sheet, cx, cy, inner_w, &style);
                    if style.flex_direction == "row" {
                        cx += child_box.w + gap;
                        max_child_w = max_child_w.max(cx - x);
                        total_h = total_h.max(child_box.h + pad * 2.0 + margin * 2.0);
                    } else {
                        cy += child_box.h + gap;
                        max_child_w = max_child_w.max(child_box.w);
                        total_h += child_box.h + gap;
                    }
                    children_layout.push(child_box);
                }
            } else {
                for child in &node.children {
                    let child_box =
                        Self::layout_node(child, sheet, x + margin + pad, cy, inner_w, &style);
                    cy += child_box.h + gap;
                    max_child_w = max_child_w.max(child_box.w);
                    total_h += child_box.h + gap;
                    children_layout.push(child_box);
                }
            }
        }

        let w = if node.tag == "canvas" {
            inner_w + pad * 2.0 + margin * 2.0
        } else if style.width == "auto" {
            max_child_w + pad * 2.0 + margin * 2.0
        } else {
            inner_w + pad * 2.0 + margin * 2.0
        };
        let h = if node.tag == "canvas" {
            node.attributes
                .get("height")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(150.0)
                + pad * 2.0
                + margin * 2.0
        } else if style.height == "auto" {
            total_h.max(24.0)
        } else {
            parse_px(&style.height, total_h)
        };

        LayoutBox {
            node_id: node.id,
            tag: node.tag.clone(),
            x,
            y,
            w,
            h,
            children: children_layout,
            text_layout: None,
        }
    }
}

fn parse_px(s: &str, default: f64) -> f64 {
    let s = s.trim();
    if s.ends_with("px") {
        s[..s.len() - 2].trim().parse().unwrap_or(default)
    } else if s.ends_with('%') {
        let pct: f64 = s[..s.len() - 1].trim().parse().unwrap_or(100.0);
        default * pct / 100.0
    } else {
        s.parse().unwrap_or(default)
    }
}
