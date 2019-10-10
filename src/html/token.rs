use std::borrow::Borrow;
use std::collections::HashMap;
use std::iter::Peekable;

// Token is a significant grouping of characters.
// Token literal is generic over anything that can be represented as a string.
#[derive(Debug, PartialEq, Clone)]
pub struct Token<K, L>
where
    K: Borrow<str>,
    L: Borrow<str>,
{
    pub kind: Kind<K>,
    pub literal: L,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Kind<K>
where
    K: Borrow<str>,
{
    OpenTag {
        name: K,
        attributes: HashMap<String, K>,
    },
    CloseTag {
        name: K,
    },
    Text(K),
}

/// Tokenizer converts a char stream into a token stream.
pub struct Tokenizer<Src>
where
    Src: Iterator<Item = char>,
{
    source: Peekable<Src>,
    current: char,
    buffer: Vec<Token<String, String>>,
    stack: Vec<char>,
}

impl<Src> Tokenizer<Src>
where
    Src: Iterator<Item = char>,
{
    pub fn new(source: Src) -> Self {
        Tokenizer {
            source: source.peekable(),
            current: '0',
            buffer: vec![],
            stack: vec![],
        }
    }
    /// merged adapts Tokenizer to an iterator that merges adjacent text tokens.
    pub fn merged(self) -> TextMerger<Tokenizer<Src>> {
        TextMerger {
            source: self.peekable(),
        }
    }
    // advance the current token, returning false if there are no more values.
    fn advance(&mut self) -> bool {
        if let Some(c) = self.source.next() {
            self.current = c;
            true
        } else {
            false
        }
    }
    // peek the next token without advancing to it.
    fn peek(&mut self) -> Option<&char> {
        self.source.peek()
    }
}

impl<Src> Iterator for Tokenizer<Src>
where
    Src: Iterator<Item = char>,
{
    type Item = Token<String, String>;

    /// next returns the next xml token in the sequence.
    ///
    /// Buffer chars until '>' is found. The buffer is processed to yield one
    /// or more tokens.
    fn next(&mut self) -> Option<Self::Item> {
        // Drain the buffer before processing more characters.
        if !self.buffer.is_empty() {
            return self.buffer.pop();
        }
        // Collect chars until we hit '>'.
        let mut stack: Vec<char> = vec![];
        while let Some(current) = self.source.next() {
            stack.push(current);
            // We begin to unwind the stack.
            if current == '>' {
                // Unwind the stack.
                let mut buffer: Vec<char> = vec![];
                while let Some(c) = stack.pop() {
                    buffer.push(c);
                    // If angle bracket, we might have tag.
                    if c == '<' {
                        // we have a buffer containing "<*.>"
                        // try parse as open tag, close tag, else text
                        let buffer: String = buffer.drain(..).rev().collect();
                        if buffer.starts_with("</") {
                            self.buffer.push(Token {
                                kind: Kind::CloseTag {
                                    name: buffer
                                        .trim_start_matches("</")
                                        .trim_end_matches(">")
                                        .trim()
                                        .to_owned(),
                                },
                                literal: buffer,
                            });
                        } else {
                            let mut words = buffer
                                .trim_start_matches('<')
                                .trim_start_matches('!')
                                .trim_start_matches('/')
                                .trim_end_matches('>')
                                .trim_end_matches('/')
                                .split_whitespace()
                                .map(String::from)
                                .collect::<Vec<String>>();
                            // is_tag if there are words that do not contain "=\"", and
                            // also contain non-alphabetic chars.
                            // If the word contains "=\"" we have an attribute value
                            // that can contain arbitrary chars, hence we can't simply
                            // look for non-alphabetic chars.
                            let is_tag = words.len() > 0
                                && words.iter().fold(true, |is_tag, word| {
                                    if !is_tag {
                                        return false;
                                    }
                                    if !word.contains("=\"")
                                        && word.contains(|c: char| !c.is_alphabetic())
                                    {
                                        false
                                    } else {
                                        true
                                    }
                                });
                            if is_tag {
                                let mut words = words.drain(..);
                                let name = words.next().unwrap();
                                let attributes: HashMap<String, String> = words
                                    .map(|attr: String| {
                                        let mut parts = attr.split("=");
                                        let name = parts.next().unwrap();
                                        let value = parts
                                            .next()
                                            .unwrap_or("")
                                            .trim_start_matches('"')
                                            .trim_end_matches('"');
                                        (name.to_owned(), value.to_owned())
                                    })
                                    .collect();
                                self.buffer.push(Token {
                                    kind: Kind::OpenTag { name, attributes },
                                    literal: buffer,
                                });
                            } else {
                                self.buffer.push(Token {
                                    kind: Kind::Text(buffer.clone()),
                                    literal: buffer,
                                });
                            }
                        }
                    }
                }
                if buffer.len() > 0 {
                    let buffer: String = buffer.drain(..).rev().collect();
                    self.buffer.push(Token {
                        kind: Kind::Text(buffer.clone()),
                        literal: buffer,
                    });
                }
                return self.buffer.pop();
            }
        }
        // If we are here then we hit EOF without hitting '>'.
        // Lets return any chars buffered as a text token.
        if !stack.is_empty() {
            let text: String = stack.drain(..).collect();
            Some(Token {
                kind: Kind::Text(text.clone()),
                literal: text,
            })
        } else {
            None
        }
    }
}

/// TextMerger merges adjacent Text Tokens into one Text Token.
pub struct TextMerger<Src>
where
    Src: Iterator<Item = Token<String, String>>,
{
    source: Peekable<Src>,
}

impl<Src> Iterator for TextMerger<Src>
where
    Src: Iterator<Item = Token<String, String>>,
{
    type Item = Token<String, String>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.source.next() {
            Some(Token {
                kind: Kind::Text(mut text),
                mut literal,
            }) => {
                // While the next token is a Text token, merge into this one.
                while let Some(Token {
                    kind: Kind::Text(_),
                    ..
                }) = self.source.peek()
                {
                    if let Some(Token {
                        kind: Kind::Text(next_text),
                        literal: next_literal,
                    }) = self.source.next()
                    {
                        text.extend(next_text.chars());
                        literal.extend(next_literal.chars());
                    }
                }
                Some(Token {
                    kind: Kind::Text(text),
                    literal,
                })
            }
            Some(other) => Some(other),
            None => None,
        }
    }
}

impl<K, L> Token<K, L>
where
    K: Borrow<str>,
    L: Borrow<str>,
{
    pub fn to_owned(&self) -> Token<String, String> {
        Token {
            kind: match &self.kind {
                Kind::OpenTag { name, attributes } => Kind::OpenTag {
                    name: name.borrow().to_string(),
                    attributes: attributes
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.borrow().to_string()))
                        .collect(),
                },
                Kind::CloseTag { name } => Kind::CloseTag {
                    name: name.borrow().to_string(),
                },
                Kind::Text(text) => Kind::Text(text.borrow().to_string()),
            },
            literal: self.literal.borrow().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn map<'a>(pairs: &[(&str, &'a str)]) -> HashMap<String, &'a str> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn tokenizer() {
        let tests = vec![
            (
                "self closing tag",
                "<first/>text<second />",
                vec![
                    Token {
                        kind: Kind::OpenTag {
                            name: "first".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<first/>",
                    },
                    Token {
                        kind: Kind::Text("text".into()),
                        literal: "text",
                    },
                    Token {
                        kind: Kind::OpenTag {
                            name: "second".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<second />",
                    },
                ],
            ),
            (
                "empty tag",
                "<tag></tag>",
                vec![
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::CloseTag { name: "tag".into() },
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "tag with attributes",
                r#"<tag one/><tag one two="two"/><tag one two="two"></tag>"#,
                vec![
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: map(&[("one", "")]),
                        },
                        literal: r#"<tag one/>"#,
                    },
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: map(&[("one", ""), ("two", "two")]),
                        },
                        literal: r#"<tag one two="two"/>"#,
                    },
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: map(&[("one", ""), ("two", "two")]),
                        },
                        literal: r#"<tag one two="two">"#,
                    },
                    Token {
                        kind: Kind::CloseTag { name: "tag".into() },
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "tag with attributes - whitespace before end of open tag",
                r#"<tag one /><tag one two="two" /><tag one two="two" ></tag>"#,
                vec![
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: map(&[("one", "")]),
                        },
                        literal: r#"<tag one />"#,
                    },
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: map(&[("one", ""), ("two", "two")]),
                        },
                        literal: r#"<tag one two="two" />"#,
                    },
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: map(&[("one", ""), ("two", "two")]),
                        },
                        literal: r#"<tag one two="two" >"#,
                    },
                    Token {
                        kind: Kind::CloseTag { name: "tag".into() },
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "simple text",
                "text",
                vec![Token {
                    kind: Kind::Text("text".into()),
                    literal: "text",
                }],
            ),
            (
                "tag containing text",
                "<tag>text</tag><tag> text </tag>",
                vec![
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text("text".into()),
                        literal: "text",
                    },
                    Token {
                        kind: Kind::CloseTag { name: "tag".into() },
                        literal: "</tag>",
                    },
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text(" text ".into()),
                        literal: " text ",
                    },
                    Token {
                        kind: Kind::CloseTag { name: "tag".into() },
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "tag containing text and tags",
                "<tag>text<tag/>text<tag>text</tag>text</tag>",
                vec![
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text("text".into()),
                        literal: "text",
                    },
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<tag/>",
                    },
                    Token {
                        kind: Kind::Text("text".into()),
                        literal: "text",
                    },
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text("text".into()),
                        literal: "text",
                    },
                    Token {
                        kind: Kind::CloseTag { name: "tag".into() },
                        literal: "</tag>",
                    },
                    Token {
                        kind: Kind::Text("text".into()),
                        literal: "text",
                    },
                    Token {
                        kind: Kind::CloseTag { name: "tag".into() },
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "doctype",
                "<!DOCTYPE html>",
                vec![Token {
                    kind: Kind::OpenTag {
                        name: "DOCTYPE".into(),
                        attributes: map(&[("html", "")]),
                    },
                    literal: "<!DOCTYPE html>",
                }],
            ),
            (
                "text with angle brackets",
                "if (foo < bar || bar > foo) {throw new Error()}",
                vec![Token {
                    kind: Kind::Text("if (foo < bar || bar > foo) {throw new Error()}".into()),
                    literal: "if (foo < bar || bar > foo) {throw new Error()}",
                }],
            ),
            (
                "text: no whitespace around angle brackets",
                "if (foo<bar || bar>foo) {throw new Error()}",
                vec![Token {
                    kind: Kind::Text("if (foo<bar || bar>foo) {throw new Error()}".into()),
                    literal: "if (foo<bar || bar>foo) {throw new Error()}",
                }],
            ),
            (
                "script containing left arrow",
                r#"<script>if (1 < 2) {alert("hi");}if (1 < 2) {alert("hi");}</script>"#,
                vec![
                    Token {
                        kind: Kind::OpenTag {
                            name: "script".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<script>",
                    },
                    Token {
                        kind: Kind::Text(
                            r#"if (1 < 2) {alert("hi");}if (1 < 2) {alert("hi");}"#.into(),
                        ),
                        literal: r#"if (1 < 2) {alert("hi");}if (1 < 2) {alert("hi");}"#,
                    },
                    Token {
                        kind: Kind::CloseTag {
                            name: "script".into(),
                        },
                        literal: "</script>",
                    },
                ],
            ),
            (
                "arbitrary number of angle brackets in a text block",
                "<tag><><<<<<>>>>><<><><><><<> asdfajal;skjdf <<> >  >> <> <>><><</tag>",
                vec![
                    Token {
                        kind: Kind::OpenTag {
                            name: "tag".into(),
                            attributes: HashMap::new(),
                        },
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text(
                            "<><<<<<>>>>><<><><><><<> asdfajal;skjdf <<> >  >> <> <>><><".into(),
                        ),
                        literal: "<><<<<<>>>>><<><><><><<> asdfajal;skjdf <<> >  >> <> <>><><",
                    },
                    Token {
                        kind: Kind::CloseTag { name: "tag".into() },
                        literal: "</tag>",
                    },
                ],
            ),
        ];
        for (desc, input, want) in tests {
            let got: Vec<Token<_, _>> = Tokenizer::new(input.chars()).merged().collect();
            let want: Vec<Token<_, _>> = want.into_iter().map(|t| t.to_owned()).collect();
            assert_eq!(want, got, "{}", desc,);
        }
    }
}
