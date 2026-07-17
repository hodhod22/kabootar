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

        // C4: only explicit flex/grid — do not treat every div as flex.
        let is_flex = style.display == "flex" || style.display == "inline-flex" || node.tag == "body";
        let is_grid = style.display == "grid";
        let gap = parse_px(&style.gap, 8.0);

        if node.tag != "canvas" {
            if is_grid {
                let cols = parse_grid_columns(&style.grid_template_columns).max(1);
                let col_w = ((inner_w - gap * (cols as f64 - 1.0)) / cols as f64).max(1.0);
                let mut col = 0usize;
                let mut row_y = cy;
                let mut row_h = 0.0f64;
                let origin_x = x + margin + pad;
                for child in &node.children {
                    let cx = origin_x + col as f64 * (col_w + gap);
                    let child_box = Self::layout_node(child, sheet, cx, row_y, col_w, &style);
                    row_h = row_h.max(child_box.h);
                    max_child_w = max_child_w.max(cx + child_box.w - x);
                    children_layout.push(child_box);
                    col += 1;
                    if col >= cols {
                        col = 0;
                        row_y += row_h + gap;
                        total_h += row_h + gap;
                        row_h = 0.0;
                    }
                }
                if col != 0 {
                    total_h += row_h + gap;
                }
            } else if is_flex {
                let mut placed = Vec::new();
                let cx = x + margin + pad;
                let mut used_main = 0.0f64;
                for child in &node.children {
                    let child_box = Self::layout_node(child, sheet, cx, cy, inner_w, &style);
                    if style.flex_direction == "row" {
                        used_main += child_box.w;
                        max_child_w = max_child_w.max(used_main);
                        total_h = total_h.max(child_box.h + pad * 2.0 + margin * 2.0);
                    } else {
                        used_main += child_box.h;
                        max_child_w = max_child_w.max(child_box.w);
                        total_h += child_box.h + gap;
                    }
                    placed.push(child_box);
                }
                let n = placed.len().max(1) as f64;
                let gaps_total = gap * (n - 1.0);
                let free = if style.flex_direction == "row" {
                    (inner_w - used_main - gaps_total).max(0.0)
                } else {
                    0.0
                };
                let (start_extra, between) = match style.justify_content.as_str() {
                    "center" => (free / 2.0, 0.0),
                    "flex-end" | "end" => (free, 0.0),
                    "space-between" if n > 1.0 => (0.0, free / (n - 1.0)),
                    _ => (0.0, 0.0),
                };
                let mut cursor = if style.flex_direction == "row" {
                    x + margin + pad + start_extra
                } else {
                    cy
                };
                for mut child_box in placed {
                    if style.flex_direction == "row" {
                        child_box.x = cursor;
                        if style.align_items == "center" {
                            let cross = (total_h - pad * 2.0 - margin * 2.0 - child_box.h).max(0.0);
                            child_box.y = cy + cross / 2.0;
                        }
                        cursor += child_box.w + gap + between;
                    } else {
                        child_box.y = cursor;
                        cursor += child_box.h + gap;
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

fn parse_grid_columns(spec: &str) -> usize {
    let spec = spec.trim();
    if spec.is_empty() {
        return 1;
    }
    spec.split_whitespace().filter(|t| !t.is_empty()).count().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::kabootar_dom::DomNode;
    use crate::runtime::kstyle::Stylesheet;

    #[test]
    fn flex_row_justify_center_shifts_children() {
        let mut root = DomNode::element("div");
        root.set_attr("style", "display:flex;flex-direction:row;justify-content:center;width:200px;gap:0;padding:0;margin:0");
        let mut a = DomNode::element("span");
        a.set_attr("style", "width:20px;height:10px;padding:0;margin:0");
        let mut b = DomNode::element("span");
        b.set_attr("style", "width:20px;height:10px;padding:0;margin:0");
        root.append(a);
        root.append(b);
        let sheet = Stylesheet { rules: vec![] };
        let layout = LayoutEngine::layout(&root, &sheet, 200.0);
        assert_eq!(layout.children.len(), 2);
        assert!(
            layout.children[0].x > layout.x + 1.0,
            "expected centered child x > root, got {}",
            layout.children[0].x
        );
    }

    #[test]
    fn grid_two_columns_places_second_beside_first() {
        let mut root = DomNode::element("div");
        root.set_attr(
            "style",
            "display:grid;grid-template-columns:1fr 1fr;width:200px;gap:0;padding:0;margin:0",
        );
        root.append(DomNode::element("span"));
        root.append(DomNode::element("span"));
        let sheet = Stylesheet { rules: vec![] };
        let layout = LayoutEngine::layout(&root, &sheet, 200.0);
        assert_eq!(layout.children.len(), 2);
        assert!(layout.children[1].x > layout.children[0].x);
    }

    #[test]
    fn block_div_is_not_implicit_flex() {
        let mut root = DomNode::element("div");
        root.set_attr("style", "display:block;width:100px;padding:0;margin:0;gap:0");
        root.append(DomNode::element("span"));
        root.append(DomNode::element("span"));
        let sheet = Stylesheet { rules: vec![] };
        let layout = LayoutEngine::layout(&root, &sheet, 100.0);
        assert_eq!(layout.children.len(), 2);
        // Block stacks vertically — same x.
        assert!((layout.children[0].x - layout.children[1].x).abs() < 0.1);
        assert!(layout.children[1].y > layout.children[0].y);
    }
}
