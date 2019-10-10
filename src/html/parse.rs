use std::collections::HashMap;
use std::fmt;
use std::iter::Peekable;

use super::token::{self, Kind};

type Token = token::Token<String, String>;

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
}

impl Node {
    fn self_closing(name: String, attributes: HashMap<String, String>) -> Self {
        Node::Tag {
            name: name,
            attributes: attributes,
            children: vec![],
        }
    }
}

/// Parser maintains state required for parsing.
#[derive(Debug)]
pub struct Parser<Src>
where
    Src: Iterator<Item = Token>,
{
    source: Peekable<Src>,
}

impl<Src> Parser<Src>
where
    Src: Iterator<Item = Token>,
{
    pub fn new(source: Src) -> Self {
        Parser {
            source: source.peekable(),
        }
    }
    pub fn parse(&mut self) -> Result<Dom, String> {
        let mut nodes: Vec<Node> = vec![];
        while let Some(token) = self.source.next() {
            if let Some(node) = self.parse_node(token)? {
                nodes.extend(node);
            }
        }
        Ok(Dom { nodes })
    }

    fn parse_node(&mut self, current: Token) -> Result<Option<Vec<Node>>, String> {
        match current.kind {
            Kind::Text(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    Ok(Some(vec![Node::Text(text.to_owned())]))
                } else {
                    Ok(None)
                }
            }
            Kind::CloseTag { name } => Err(format!("unexpected close tag: </{}>", name)),
            Kind::OpenTag {
                name: open_name,
                attributes,
            } => {
                let is_self_closing = current.literal.ends_with("/>");
                if is_self_closing {
                    Ok(Some(vec![Node::self_closing(open_name, attributes)]))
                } else {
                    let mut nodes: Vec<Node> = vec![];
                    while let Some(token) = self.source.next() {
                        match token.kind {
                            Kind::CloseTag { name: close_name } => {
                                // If we encounter a close tag that doesn't
                                // match the open tag, then we have an unclosed
                                // tag. Thus the currently parsed nodes are
                                // siblings, not children.
                                if open_name != close_name {
                                    return Ok(Some(
                                        vec![Node::Tag {
                                            name: open_name,
                                            attributes: attributes,
                                            children: vec![],
                                        }]
                                        .into_iter()
                                        .chain(nodes.drain(..))
                                        .collect(),
                                    ));
                                } else {
                                    return Ok(Some(vec![Node::Tag {
                                        name: open_name,
                                        attributes: attributes,
                                        children: nodes.drain(..).collect(),
                                    }]));
                                }
                            }
                            _ => {
                                // We say this node is likely a child.
                                if let Some(n) = self.parse_node(token)? {
                                    nodes.extend(n);
                                }
                            }
                        };
                    }
                    // So, are the nodes children or siblings?
                    // How to tell? They are siblings IF there is no
                    // corresponding close tag.
                    println!("children {:?}", nodes);
                    return Ok(Some(vec![Node::Tag {
                        name: open_name,
                        attributes: attributes,
                        children: nodes.drain(..).collect(),
                    }]));
                }
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
    enum Error {
        Yes,
        No,
    }
    #[test]
    fn parser() {
        let tests = vec![
            (
                "tag mismatch, close tag without coresponding open tag",
                r#"
                <outer>
                    text
                    </inner>
                </outer>
                "#
                .trim(),
                vec![],
                Error::Yes,
            ),
            (
                "tag mismatch, open tag without coresponding close tag",
                r#"
                <outer>
                    <inner>
                    text
                </outer>
                "#
                .trim(),
                vec![Node::Tag {
                    name: "outer".into(),
                    attributes: HashMap::new(),
                    children: vec![
                        Node::Tag {
                            name: "inner".into(),
                            attributes: HashMap::new(),
                            children: vec![],
                        },
                        Node::Text("text".into()),
                    ],
                }],
                Error::No,
            ),
            (
                "script containing left arrow",
                r#"<script>if (1 < 2) {alert("hi");}</script>"#,
                vec![Node::Tag {
                    name: "script".into(),
                    attributes: HashMap::new(),
                    children: vec![Node::Text(r#"if (1 < 2) {alert("hi");}"#.into())],
                }],
                Error::No,
            ),
            (
                "minimal",
                "<tag/>",
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![],
                }],
                Error::No,
            ),
            (
                "minimal, space after tag name",
                "<tag />",
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![],
                }],
                Error::No,
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
                Error::No,
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
                Error::No,
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
                Error::No,
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
                Error::No,
            ),
            (
                "value attributes, self closing",
                r#"<tag one="foo" two="foo" three="foo"/>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: [("one", "foo"), ("two", "foo"), ("three", "foo")]
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    children: vec![],
                }],
                Error::No,
            ),
            (
                "value attributes, not self closing",
                r#"<tag one="foo" two="foo" three="foo"></tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: [("one", "foo"), ("two", "foo"), ("three", "foo")]
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    children: vec![],
                }],
                Error::No,
            ),
            (
                "full tag, empty",
                r#"<tag></tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![],
                }],
                Error::No,
            ),
            (
                "text content",
                r#"<tag>text</tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![Node::Text("text".into())],
                }],
                Error::No,
            ),
            (
                "text content, trim whitespace padding",
                r#"<tag>  text  </tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![Node::Text("text".into())],
                }],
                Error::No,
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
                Error::No,
            ),
            (
                "node content, multi child",
                r#"
                <tag>
                    <tag one="foo"/>
                    <tag>text</tag>
                </tag>
                "#
                .trim(),
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
                            children: vec![Node::Text("text".into())],
                        },
                    ],
                }],
                Error::No,
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
                Error::No,
            ),
            (
                "doctype",
                r#"<!DOCTYPE html>"#,
                vec![Node::Tag {
                    name: "!DOCTYPE".into(),
                    attributes: [("html", "")]
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    children: vec![],
                }],
                Error::No,
            ),
        ];
        for (desc, input, want, err) in tests {
            let got = Parser::new(Tokenizer::new(input.chars()).merged()).parse();
            match err {
                Error::Yes => {
                    if let Ok(got) = got {
                        assert_eq!(Dom { nodes: want }, got, "{}: wanted error, got none", desc);
                    }
                }
                Error::No => match got {
                    Ok(got) => assert_eq!(Dom { nodes: want }, got, "{}", desc),
                    Err(err) => panic!("unexpected error: {:?}", err),
                },
            };
        }
    }
}
