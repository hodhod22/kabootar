//! Paint Kabootar DOM layout to styled HTML and text preview.

use crate::runtime::kabootar_dom::DomNode;
use crate::runtime::kstyle::{compute_style, Stylesheet};
use crate::runtime::render::layout::LayoutBox;

pub fn paint_frame_html(
    root: &DomNode,
    sheet: &Stylesheet,
    layout: &LayoutBox,
    viewport_w: f64,
    viewport_h: f64,
) -> String {
    let mut body = String::new();
    paint_node_html(root, layout, sheet, &mut body);
    format!(
        r#"<!DOCTYPE html>
<html lang="sv"><head><meta charset="UTF-8">
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:"Segoe UI",system-ui,sans-serif;background:#202124;overflow:hidden}}
.kb-viewport{{position:relative;width:{vw}px;height:{vh}px;overflow:auto;background:#292a2d}}
.kb-node{{position:absolute;overflow:hidden}}
.kb-text{{white-space:pre-wrap;word-break:break-word}}
</style></head><body>
<div class="kb-viewport" data-kb-layer="kabootar">{body}</div>
</body></html>"#,
        vw = viewport_w as i64,
        vh = viewport_h as i64,
        body = body
    )
}

fn paint_node_html(node: &DomNode, layout: &LayoutBox, sheet: &Stylesheet, out: &mut String) {
    if node.tag == "#text" {
        let text = node.text.as_deref().unwrap_or("");
        let style = compute_style("span", &node.attributes, sheet);
        out.push_str(&format!(
            r#"<span class="kb-node kb-text" data-kb-id="{}" style="left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;color:{};font-size:{};line-height:{};white-space:{};">{}</span>"#,
            node.id,
            layout.x,
            layout.y,
            layout.w,
            layout.h,
            style.color,
            style.font_size,
            style.line_height,
            style.white_space,
            escape_html(text)
        ));
        return;
    }

    let style = compute_style(&node.tag, &node.attributes, sheet);
    let inline = style.to_inline_css();
    let events = node
        .listeners
        .keys()
        .map(|e| format!(" data-kb-event-{e}=\"true\""))
        .collect::<String>();

    out.push_str(&format!(
        r#"<div class="kb-node" data-kb-id="{}" data-kb-tag="{}"{}{} style="left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;{}"></div>"#,
        node.id,
        node.tag,
        events,
        if node.listeners.contains_key("click") {
            " data-kb-clickable=\"true\""
        } else {
            ""
        },
        layout.x,
        layout.y,
        layout.w,
        layout.h,
        inline
    ));

    for (child, child_layout) in node.children.iter().zip(layout.children.iter()) {
        paint_node_html(child, child_layout, sheet, out);
    }
}

pub fn paint_text_preview(root: &DomNode, layout: &LayoutBox) -> String {
    let mut lines = Vec::new();
    text_walk(root, layout, 0, &mut lines);
    lines.join("\n")
}

fn text_walk(node: &DomNode, layout: &LayoutBox, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    if node.tag == "#text" {
        if let Some(t) = &node.text {
            if !t.trim().is_empty() {
                lines.push(format!("{}{}", indent, t.trim()));
            }
        }
        return;
    }
    lines.push(format!(
        "{}{}#{} [{:.0}x{:.0}]",
        indent, node.tag, node.id, layout.w, layout.h
    ));
    for (c, cl) in node.children.iter().zip(layout.children.iter()) {
        text_walk(c, cl, depth + 1, lines);
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
