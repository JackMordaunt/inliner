use std::iter::Peekable;

// Token is a significant grouping of characters.
// Token literal is generic over anything that can be represented as a string.
#[derive(Debug, PartialEq)]
pub struct Token<L>
where
    L: AsRef<str>,
{
    pub kind: Kind,
    pub literal: L,
}

#[derive(Debug, PartialEq)]
pub enum Kind {
    OpenTag,
    CloseTag,
    Text,
}

/// Tokenizer converts a char stream into a token stream.
pub struct Tokenizer<Src>
where
    Src: Iterator<Item = char>,
{
    source: Peekable<Src>,
    current: char,
}

impl<Src> Tokenizer<Src>
where
    Src: Iterator<Item = char>,
{
    pub fn new(mut source: Src) -> Result<Self, String> {
        let current = match source.next() {
            Some(c) => c,
            None => return Err("cannot tokenize empty source".into()),
        };
        Ok(Tokenizer {
            source: source.peekable(),
            current: current,
        })
    }
    // collect chars into a string buffer until the needle is found.
    // The buffer will contain the needle.
    fn collect_including(&mut self, needle: char) -> String {
        let mut buffer: String = self.current.to_string();
        while self.advance() {
            buffer.push(self.current);
            if self.current == needle {
                break;
            }
        }
        self.advance();
        buffer
    }
    // collect chars into a string buffer until the needle is found.
    // The buffer will not include the needle.
    fn collect_until(&mut self, needle: char) -> String {
        let mut buffer: String = self.current.to_string();
        while self.advance() {
            if self.current == needle {
                break;
            }
            buffer.push(self.current);
        }
        buffer
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
    type Item = Token<String>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.peek().is_none() {
            return None;
        }
        match self.current {
            // 1. OpenTag   "<tag>"
            // 2. CloseTag  "</tag>"
            // 3. Text      "if (2 < 3) { }"
            '<' => match self.peek() {
                // "</" is the start of a close tag.
                Some('/') => Some(Token {
                    kind: Kind::CloseTag,
                    literal: self.collect_including('>'),
                }),
                Some(_) => {
                    let lit = self.collect_including('>');
                    // is_tag if there are words that do not contain "=\"", and
                    // also contain non-alphabetic chars.
                    // If the word contains "=\"" we have an attribute value
                    // that can contain arbitrary chars, hence we can't simply
                    // look for non-alphabetic chars.
                    let is_tag = lit
                        .trim_start_matches('<')
                        .trim_start_matches('!') // <!DOCTYPE html>
                        .trim_end_matches('>')
                        .trim_end_matches('/') // <tag foo="bar" />
                        .split_ascii_whitespace()
                        .fold(true, |is_tag, word| {
                            if !is_tag {
                                return false;
                            }
                            if !word.contains("=\"") && word.contains(|c: char| !c.is_alphabetic())
                            {
                                false
                            } else {
                                true
                            }
                        });
                    if is_tag {
                        Some(Token {
                            kind: Kind::OpenTag,
                            literal: lit,
                        })
                    } else {
                        Some(Token {
                            kind: Kind::Text,
                            literal: lit,
                        })
                    }
                }
                _ => None,
            },
            _ => Some(Token {
                kind: Kind::Text,
                literal: self.collect_until('<'),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    #[test]
    fn tokenizer() {
        let tests = vec![
            (
                "self closing tag",
                "<tag/><tag />",
                vec![
                    Token {
                        kind: Kind::OpenTag,
                        literal: "<tag/>",
                    },
                    Token {
                        kind: Kind::OpenTag,
                        literal: "<tag />",
                    },
                ],
            ),
            (
                "empty tag",
                "<tag></tag>",
                vec![
                    Token {
                        kind: Kind::OpenTag,
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::CloseTag,
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "tag with attributes",
                r#"<tag one/><tag one two="two"/><tag one two="two"></tag>"#,
                vec![
                    Token {
                        kind: Kind::OpenTag,
                        literal: r#"<tag one/>"#,
                    },
                    Token {
                        kind: Kind::OpenTag,
                        literal: r#"<tag one two="two"/>"#,
                    },
                    Token {
                        kind: Kind::OpenTag,
                        literal: r#"<tag one two="two">"#,
                    },
                    Token {
                        kind: Kind::CloseTag,
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "tag with attributes - whitespace before end of open tag",
                r#"<tag one /><tag one two="two" /><tag one two="two" ></tag>"#,
                vec![
                    Token {
                        kind: Kind::OpenTag,
                        literal: r#"<tag one />"#,
                    },
                    Token {
                        kind: Kind::OpenTag,
                        literal: r#"<tag one two="two" />"#,
                    },
                    Token {
                        kind: Kind::OpenTag,
                        literal: r#"<tag one two="two" >"#,
                    },
                    Token {
                        kind: Kind::CloseTag,
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "simple text",
                "text",
                vec![Token {
                    kind: Kind::Text,
                    literal: "text",
                }],
            ),
            (
                "tag containing text",
                "<tag>text</tag><tag> text </tag>",
                vec![
                    Token {
                        kind: Kind::OpenTag,
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: "text",
                    },
                    Token {
                        kind: Kind::CloseTag,
                        literal: "</tag>",
                    },
                    Token {
                        kind: Kind::OpenTag,
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: " text ",
                    },
                    Token {
                        kind: Kind::CloseTag,
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "tag containing text and tags",
                "<tag>text<tag/>text<tag>text</tag>text</tag>",
                vec![
                    Token {
                        kind: Kind::OpenTag,
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: "text",
                    },
                    Token {
                        kind: Kind::OpenTag,
                        literal: "<tag/>",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: "text",
                    },
                    Token {
                        kind: Kind::OpenTag,
                        literal: "<tag>",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: "text",
                    },
                    Token {
                        kind: Kind::CloseTag,
                        literal: "</tag>",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: "text",
                    },
                    Token {
                        kind: Kind::CloseTag,
                        literal: "</tag>",
                    },
                ],
            ),
            (
                "doctype",
                "<!DOCTYPE html>",
                vec![Token {
                    kind: Kind::OpenTag,
                    literal: "<!DOCTYPE html>",
                }],
            ),
            (
                "text with angle brackets",
                "if (foo < bar || bar > foo) {throw new Error()}",
                vec![
                    Token {
                        kind: Kind::Text,
                        literal: "if (foo ",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: "< bar || bar >",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: " foo) {throw new Error()}",
                    },
                ],
            ),
            (
                "text: no whitespace around angle brackets",
                "if (foo<bar || bar>foo) {throw new Error()}",
                vec![
                    Token {
                        kind: Kind::Text,
                        literal: "if (foo",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: "<bar || bar>",
                    },
                    Token {
                        kind: Kind::Text,
                        literal: "foo) {throw new Error()}",
                    },
                ],
            ),
        ];
        for (desc, input, want) in tests {
            let got: Vec<Token<String>> = Tokenizer::new(input.chars()).unwrap().collect();
            let want = want
                .into_iter()
                .map(|tok: Token<&str>| Token {
                    literal: tok.literal.to_owned(),
                    kind: tok.kind,
                })
                .collect::<Vec<Token<String>>>();
            assert_eq!(want, got, "{}", desc,);
        }
    }
}
