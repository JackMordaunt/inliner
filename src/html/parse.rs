use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::iter::Peekable;
use std::rc::Rc;

use super::token::{self, Kind};

/// NodeRef is used for interior mutability, enabling mutations of the DOM
/// during traversal.
pub type NodeRef = Rc<RefCell<Node>>;
type Token = token::Token<String, String>;

/// Dom is a simple wrapper over the root level Nodes.
#[derive(Debug, PartialEq)]
pub struct Dom {
    pub nodes: Vec<NodeRef>,
}

/// Node defines what data can appear in the DOM tree.
#[derive(Debug, PartialEq)]
pub enum Node {
    Text(String),
    Tag {
        name: String,
        attributes: HashMap<String, String>,
        children: Vec<NodeRef>,
    },
}

/// Parser maintains state required for parsing.
#[derive(Debug)]
pub struct Parser<Src>
where
    Src: Iterator<Item = Token>,
{
    source: Peekable<Src>,
}

impl Dom {
    /// depth_first recursively walks the DOM depth_first, applying `cb` on
    /// every Node.
    /// Errors in the callback will bubble up here so the caller can access it.
    pub fn depth_first<F>(&self, cb: &F) -> Result<(), Box<dyn Error>>
    where
        F: Fn(NodeRef) -> Result<(), Box<dyn Error>>,
    {
        Dom::visit_notes(&self.nodes, cb)
    }
    fn visit_notes<F>(nodes: &[NodeRef], cb: &F) -> Result<(), Box<dyn Error>>
    where
        F: Fn(NodeRef) -> Result<(), Box<dyn Error>>,
    {
        for node in nodes {
            cb(node.clone())?;
            if let Node::Tag { children, .. } = &*node.borrow() {
                Dom::visit_notes(children, cb)?;
            }
        }
        Ok(())
    }
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

impl<Src> Parser<Src>
where
    Src: Iterator<Item = Token>,
{
    pub fn new(source: Src) -> Self {
        Parser {
            source: source.peekable(),
        }
    }

    /// parse the token stream into a DOM tree.
    pub fn parse(&mut self) -> Result<Dom, String> {
        let mut nodes: Vec<NodeRef> = vec![];
        while let Some(token) = self.source.next() {
            if let Some(node) = self.parse_node(token)? {
                nodes.extend(node);
            }
        }
        Ok(Dom { nodes })
    }

    // parse_node recursively parses `Node` objects in depth first order.
    // Extremely nested input could overflow the stack.
    fn parse_node(&mut self, current: Token) -> Result<Option<Vec<NodeRef>>, String> {
        match current.kind {
            Kind::Text(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    Ok(Some(vec![Node::Text(text.to_owned()).into()]))
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
                    Ok(Some(vec![Node::self_closing(open_name, attributes).into()]))
                } else {
                    let mut siblings: Vec<NodeRef> = vec![];
                    while let Some(token) = self.source.peek() {
                        match &token.kind {
                            Kind::CloseTag { name: close_name } => {
                                // If we encounter a close tag that doesn't
                                // match the open tag, then we have an unclosed
                                // tag. Thus the currently parsed nodes are
                                // siblings, not children.
                                if open_name != *close_name {
                                    return Ok(Some(
                                        vec![Node::Tag {
                                            name: open_name,
                                            attributes: attributes,
                                            children: vec![],
                                        }
                                        .into()]
                                        .into_iter()
                                        .chain(siblings.drain(..))
                                        .collect(),
                                    ));
                                } else {
                                    self.source.next();
                                    return Ok(Some(vec![Node::Tag {
                                        name: open_name,
                                        attributes: attributes,
                                        children: siblings,
                                    }
                                    .into()]));
                                }
                            }
                            _ => {
                                if let Some(token) = self.source.next() {
                                    if let Some(n) = self.parse_node(token)? {
                                        siblings.extend(n);
                                    }
                                }
                            }
                        };
                    }
                    // Ran out of input before finding a close tag, so this node
                    // must be a sibling of the buffered nodes.
                    return Ok(Some(
                        vec![Node::Tag {
                            name: open_name,
                            attributes: attributes,
                            children: vec![],
                        }
                        .into()]
                        .into_iter()
                        .chain(siblings.drain(..))
                        .collect(),
                    ));
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
            self.nodes.iter().map(|n| n.borrow().to_string()).fold(
                String::new(),
                |mut acc, next| {
                    acc.extend(next.chars());
                    acc.push('\n');
                    acc
                }
            )
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
                        "<{tag}{attributes}/>",
                        tag = name,
                        attributes = attributes
                            .iter()
                            .map(|(k, v)| if !v.is_empty() {
                                format!("{}=\"{}\"", k, v)
                            } else {
                                format!("{}", k)
                            })
                            .fold(String::new(), |mut acc, next| {
                                acc.push(' ');
                                acc.extend(next.chars());
                                acc
                            })
                            .trim_end()
                    )
                } else {
                    write!(
                        f,
                        "<{tag}{attributes}>{children}</{tag}>",
                        tag = name,
                        attributes = attributes
                            .iter()
                            .map(|(k, v)| if !v.is_empty() {
                                format!("{}=\"{}\"", k, v)
                            } else {
                                format!("{}", k)
                            })
                            .fold(String::new(), |mut acc, next| {
                                acc.push(' ');
                                acc.extend(next.chars());
                                acc
                            })
                            .trim_end(),
                        children = children
                            .iter()
                            .map(|n| n.borrow().to_string())
                            .fold(String::new(), |mut acc, next| {
                                acc.push(' ');
                                acc.extend(next.chars());
                                acc
                            })
                            .trim_end()
                    )
                }
            }
        }
    }
}

impl Into<NodeRef> for Node {
    fn into(self) -> NodeRef {
        Rc::new(RefCell::new(self))
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
                // Fail symptom: Open tag without flatten into a list of siblings.
                // I think this is because we consume the </outer> when comparing it with <inner>
                // which means the <outer> reaches end of input and is considered an open tag without a close tag.
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
                        }
                        .into(),
                        Node::Text("text".into()).into(),
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
                    children: vec![Node::Text(r#"if (1 < 2) {alert("hi");}"#.into()).into()],
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
                    children: vec![Node::Text("text".into()).into()],
                }],
                Error::No,
            ),
            (
                "text content, trim whitespace padding",
                r#"<tag>  text  </tag>"#,
                vec![Node::Tag {
                    name: "tag".into(),
                    attributes: HashMap::new(),
                    children: vec![Node::Text("text".into()).into()],
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
                    }
                    .into()],
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
                        }
                        .into(),
                        Node::Tag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                            children: vec![Node::Text("text".into()).into()],
                        }
                        .into(),
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
                            children: vec![Node::Text("text".into()).into()],
                        }
                        .into()],
                    }
                    .into()],
                }],
                Error::No,
            ),
            (
                "doctype: first tag is an open tag without a close tag",
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
            (
                // Fail: Open tag without close tag fails when part of the document root.
                // Symptom: Following tag becomes child instead of sibling.
                "doctype: first tag is an open tag without a close tag",
                r#"
                <!DOCTYPE html>
                <html>
                    <body>
                    </body>
                </html>
                "#,
                vec![
                    Node::Tag {
                        name: "!DOCTYPE".into(),
                        attributes: [("html", "")]
                            .into_iter()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect(),
                        children: vec![],
                    },
                    Node::Tag {
                        name: "html".into(),
                        attributes: HashMap::new(),
                        children: vec![Node::Tag {
                            name: "body".into(),
                            attributes: HashMap::new(),
                            children: vec![],
                        }
                        .into()],
                    },
                ],
                Error::No,
            ),
        ];
        for (desc, input, mut want, err) in tests {
            let got = Parser::new(Tokenizer::new(input.chars()).merged()).parse();
            let want = want.drain(..).map(Into::into).collect();
            match err {
                Error::Yes => {
                    if let Ok(got) = got {
                        assert_eq!(Dom { nodes: want }, got, "{}: wanted error, got none", desc,);
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
