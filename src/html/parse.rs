use std::collections::HashMap;
use std::fmt;
use std::iter::Peekable;

use super::token::{Kind, Token};

#[derive(Debug, PartialEq)]
pub struct Dom {
    nodes: Vec<Node>,
}

#[derive(Debug, PartialEq)]
pub enum Node {
    Text(String),
    Tag {
        name: String,
        attributes: HashMap<String, String>,
        children: Vec<Node>,
    },
    DocType,
}

/// Parser maintains state required for parsing.
#[derive(Debug)]
pub struct Parser<Src>
where
    Src: Iterator<Item = Token>,
{
    source: Peekable<Src>,
    current: Token,
}

impl<Src> Parser<Src>
where
    Src: Iterator<Item = Token>,
{
    pub fn new(source: Src) -> Result<Self, String> {
        let mut source = source.peekable();
        let current = match source.next() {
            Some(c) => c,
            None => return Err("source is empty".to_owned()),
        };
        Ok(Parser { current, source })
    }
    pub fn parse(&mut self) -> Result<Dom, String> {
        let mut nodes: Vec<Node> = vec![];
        while let Some(_) = self.peek() {
            nodes.push(self.parse_node()?);
        }
        Ok(Dom { nodes })
    }
    fn advance(&mut self) {
        if let Some(tok) = self.source.next() {
            self.current = tok;
        }
    }
    fn peek(&mut self) -> Option<&Token> {
        self.source.peek()
    }
    fn expect(&mut self, kind: Kind) -> Result<(), String> {
        match self.peek() {
            None => Err(format!("unexpected end of input")),
            Some(tok) => {
                if tok.kind != kind {
                    Err(format!("expected {:?}, got {:?}", kind, tok))
                } else {
                    Ok(())
                }
            }
        }
    }
    fn current(&mut self, kind: Kind) -> Result<(), String> {
        if self.current.kind == kind {
            Ok(())
        } else {
            Err(format!("expected {:?}, got {:?}", kind, self.current))
        }
    }
    fn eat_whitespace(&mut self) {
        while self.current(Kind::WhiteSpace).is_ok() && self.peek().is_some() {
            self.advance();
        }
    }
    fn parse_node(&mut self) -> Result<Node, String> {
        match self.current.kind {
            Kind::LeftArrow => {
                let mut tag = String::new();
                // Parse tag name.
                while self.expect(Kind::Text).is_ok() {
                    self.advance();
                    tag.push(self.current.literal);
                }
                self.advance();
                if tag.starts_with('!') {
                    while self.current(Kind::RightArrow).is_err() {
                        self.advance();
                    }
                    self.advance();
                    return Ok(Node::DocType);
                }
                self.eat_whitespace();
                let mut attributes = HashMap::new();
                if self.current(Kind::RightArrow).is_err() {
                    while self.expect(Kind::RightArrow).is_err()
                        && self.expect(Kind::Slash).is_err()
                    {
                        self.eat_whitespace();
                        // Parse attributes.
                        let mut attr = self.current.literal.to_string();
                        while self.expect(Kind::Text).is_ok() {
                            self.advance();
                            attr.push(self.current.literal);
                        }
                        self.advance();
                        if !attr.is_empty() {
                            // Check for value.
                            if self.current(Kind::Equal).is_ok() && self.expect(Kind::Quote).is_ok()
                            {
                                self.advance();
                                let mut value = String::new();
                                while self.expect(Kind::Quote).is_err() {
                                    self.advance();
                                    value.push(self.current.literal);
                                }
                                self.advance();
                                self.advance();
                                attributes.insert(attr, value);
                            } else {
                                attributes.insert(attr, "".to_string());
                            }
                        }
                    }
                }
                self.eat_whitespace();
                if self.current(Kind::Slash).is_ok() {
                    // Self closing.
                    self.advance();
                    self.advance();
                    Ok(Node::Tag {
                        name: tag,
                        attributes: attributes,
                        children: vec![],
                    })
                } else {
                    // Parse child nodes until we hit the close tag ("</").
                    self.advance();
                    let mut children = vec![];
                    while !(self.current.kind == Kind::LeftArrow
                        && self.expect(Kind::Slash).is_ok())
                    {
                        children.push(self.parse_node()?);
                    }
                    for _ in 0..tag.len() + 3 {
                        self.advance();
                    }
                    Ok(Node::Tag {
                        name: tag,
                        attributes: attributes,
                        children: children,
                    })
                }
            }
            _ => {
                let mut text = self.current.literal.to_string();
                while self.expect(Kind::LeftArrow).is_err() {
                    self.advance();
                    text.push(self.current.literal);
                }
                self.advance();
                Ok(Node::Text(text))
            }
        }
    }
}

impl fmt::Display for Dom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.nodes
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Node::Text(text) => write!(f, "{}", text),
            Node::DocType => write!(f, "<!DOCTYPE html>"),
            Node::Tag {
                name,
                attributes,
                children,
            } => {
                if children.is_empty() {
                    write!(
                        f,
                        "<{tag} {attributes}/>",
                        tag = name,
                        attributes = format!("{:?}", attributes)
                    )
                } else {
                    write!(
                        f,
                        "<{tag} {attributes}>{children}</{tag}>",
                        tag = name,
                        attributes = format!("{:?}", attributes),
                        children = children
                            .iter()
                            .map(|n| n.to_string())
                            .collect::<Vec<String>>()
                            .join(" ")
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Tokenizer;
    use super::*;
    use pretty_assertions::assert_eq;
    #[test]
    fn tag() {
        let tests = vec![
            (
                "minimal",
                "<tag/>",
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![],
                }],
            ),
            (
                "minimal, space after tag name",
                "<tag />",
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![],
                }],
            ),
            (
                "boolean attributes",
                "<tag one two three/>",
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: [("one", ""), ("two", ""), ("three", "")]
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    children: vec![],
                }],
            ),
            (
                "boolean attributes, multiple spaces between",
                "<tag   one    two    three />",
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: [("one", ""), ("two", ""), ("three", "")]
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    children: vec![],
                }],
            ),
            (
                "boolean attributes, space after last attribute",
                "<tag one two three />",
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: [("one", ""), ("two", ""), ("three", "")]
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    children: vec![],
                }],
            ),
            (
                "value attributes",
                r#"<tag one="foo" two="foo" three="foo"/>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: [("one", "foo"), ("two", "foo"), ("three", "foo")]
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    children: vec![],
                }],
            ),
            (
                "value attributes, space after last attribute",
                r#"<tag one="foo" two="foo" three="foo" />"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: [("one", "foo"), ("two", "foo"), ("three", "foo")]
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    children: vec![],
                }],
            ),
            (
                "full tag, empty",
                r#"<tag></tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![],
                }],
            ),
            (
                "text content",
                r#"<tag>text</tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![Node::Text("text".into())],
                }],
            ),
            (
                "text content, preserve whitespace padding",
                r#"<tag>  text  </tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![Node::Text("  text  ".into())],
                }],
            ),
            (
                "node content, single child",
                r#"<tag><tag/></tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![Node::Tag {
                        name: "tag".into(),
                        attributes: HashMap::new(),
                        children: vec![],
                    }],
                }],
            ),
            (
                "node content, multi child",
                r#"<tag><tag one="foo"/><tag></tag></tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![
                        Node::Tag {
                            name: "tag".into(),
                            attributes: [("one", "foo")]
                                .into_iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                            children: vec![],
                        },
                        Node::Tag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                            children: vec![],
                        },
                    ],
                }],
            ),
            (
                "node content, nested",
                r#"<tag><tag><tag>text</tag></tag></tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![Node::Tag {
                        name: "tag".into(),
                        attributes: HashMap::new(),
                        children: vec![Node::Tag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                            children: vec![Node::Text("text".into())],
                        }],
                    }],
                }],
            ),
            ("doctype", r#"<!DOCTYPE html>"#, vec![Node::DocType]),
        ];
        for (desc, input, want) in tests {
            let got = Parser::new(Tokenizer::from(input.chars()))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(Dom { nodes: want }, got, "{}", desc);
        }
    }
}
