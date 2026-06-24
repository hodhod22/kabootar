//! Kabootar Markup Language (KML) — XML-liknande syntax för Kabootar DOM.
//!
//! ```kml
//! <div class="main">
//!   <h1>Title</h1>
//!   Hello
//! </div>
//! ```

use crate::runtime::kabootar_dom::{assign_ids, DomNode};
use std::collections::HashMap;

pub fn parse_kml(input: &str) -> Result<DomNode, String> {
    let mut parser = KmlParser::new(input);
    parser.parse_document()
}

pub fn render_kml(node: &DomNode) -> String {
    if node.tag == "#text" {
        return escape_text(node.text.as_deref().unwrap_or(""));
    }
    let mut out = String::new();
    out.push('<');
    out.push_str(&node.tag);
    for (key, value) in &node.attributes {
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(&escape_attr(value));
        out.push('"');
    }
    if node.children.is_empty() && node.text.is_none() {
        out.push_str(" />");
        return out;
    }
    out.push('>');
    if let Some(text) = &node.text {
        out.push_str(&escape_text(text));
    }
    for child in &node.children {
        out.push_str(&render_kml(child));
    }
    out.push_str("</");
    out.push_str(&node.tag);
    out.push('>');
    out
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    escape_text(s).replace('"', "&quot;")
}

struct KmlParser {
    chars: Vec<char>,
    pos: usize,
}

impl KmlParser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        self.pos += 1;
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if !c.is_whitespace() {
                break;
            }
            self.next();
        }
    }

    fn parse_document(&mut self) -> Result<DomNode, String> {
        self.skip_whitespace();
        if self.peek() != Some('<') {
            return Err("KML must start with an element".into());
        }
        let root = self.parse_element()?;
        self.skip_whitespace();
        if self.peek().is_some() {
            return Err("Unexpected content after root element".into());
        }
        Ok(root)
    }

    fn parse_element(&mut self) -> Result<DomNode, String> {
        self.expect_char('<')?;
        if self.peek() == Some('/') {
            return Err("Unexpected closing tag".into());
        }
        let tag = self.read_name()?;
        let attributes = self.parse_attributes()?;
        self.skip_whitespace();

        if self.peek() == Some('/') {
            self.next();
            self.expect_char('>')?;
            let mut node = DomNode::element(tag);
            node.attributes = attributes;
            assign_ids(&mut node);
            return Ok(node);
        }

        self.expect_char('>')?;
        let tag_name = tag.clone();
        let mut node = DomNode::element(tag);
        node.attributes = attributes;

        loop {
            self.skip_whitespace();
            if self.peek() == Some('<') {
                if self.upcoming_is_close_tag() {
                    break;
                }
                node.children.push(self.parse_element()?);
            } else if self.peek().is_some() {
                let text = self.read_text_until('<')?;
                if !text.is_empty() {
                    node.children.push(DomNode::text_node(text));
                }
            } else {
                break;
            }
        }

        self.consume_close_tag(&tag_name)?;
        assign_ids(&mut node);
        Ok(node)
    }

    fn parse_attributes(&mut self) -> Result<HashMap<String, String>, String> {
        let mut attrs = HashMap::new();
        self.skip_whitespace();
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '-' || c == '_') {
            let name = self.read_name()?;
            self.skip_whitespace();
            self.expect_char('=')?;
            self.skip_whitespace();
            let value = self.read_quoted_value()?;
            attrs.insert(name, value);
            self.skip_whitespace();
        }
        Ok(attrs)
    }

    fn read_name(&mut self) -> Result<String, String> {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                self.next();
            } else {
                break;
            }
        }
        if name.is_empty() {
            Err("Expected element or attribute name".into())
        } else {
            Ok(name)
        }
    }

    fn read_quoted_value(&mut self) -> Result<String, String> {
        match self.next() {
            Some('"') => self.read_until('"'),
            Some('\'') => self.read_until('\''),
            _ => Err("Expected quoted attribute value".into()),
        }
    }

    fn read_until(&mut self, end: char) -> Result<String, String> {
        let mut value = String::new();
        while let Some(c) = self.next() {
            if c == end {
                return Ok(value);
            }
            value.push(c);
        }
        Err(format!("Unterminated attribute value, expected '{}'", end))
    }

    fn read_text_until(&mut self, end: char) -> Result<String, String> {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c == end {
                break;
            }
            text.push(c);
            self.next();
        }
        Ok(text)
    }

    fn upcoming_is_close_tag(&self) -> bool {
        self.chars.get(self.pos + 1) == Some(&'/')
    }

    fn consume_close_tag(&mut self, expected: &str) -> Result<(), String> {
        self.expect_char('<')?;
        self.expect_char('/')?;
        let tag = self.read_name()?;
        self.skip_whitespace();
        self.expect_char('>')?;
        if tag != expected {
            return Err(format!("Closing tag </{}> does not match <{}>", tag, expected));
        }
        Ok(())
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        match self.next() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(format!("Expected '{}', found '{}'", expected, c)),
            None => Err(format!("Expected '{}', found end of input", expected)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nested_elements() {
        let node = parse_kml(r#"<div class="main"><p>Hi</p></div>"#).unwrap();
        assert_eq!(node.tag, "div");
        assert_eq!(node.attributes.get("class").map(String::as_str), Some("main"));
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].tag, "p");
    }

    #[test]
    fn render_roundtrip() {
        let input = r#"<div class="x"><h1>Title</h1></div>"#;
        let node = parse_kml(input).unwrap();
        assert_eq!(render_kml(&node), input);
    }
}
