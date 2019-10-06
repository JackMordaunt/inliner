#[derive(Debug)]
pub struct Token {
    pub kind: Kind,
    pub literal: char,
    // pos: (usize, usize),
}

#[derive(Debug, PartialEq)]
pub enum Kind {
    LeftArrow,
    RightArrow,
    Slash,
    Equal,
    Quote,
    Text,
    WhiteSpace,
}

/// Tokenizer converts a char stream into a token stream.
pub struct Tokenizer<Src>
where
    Src: Iterator<Item = char>,
{
    source: Src,
}

impl<Src> Tokenizer<Src>
where
    Src: Iterator<Item = char>,
{
    pub fn from(source: Src) -> Self {
        Tokenizer { source }
    }
}

impl<Src> Iterator for Tokenizer<Src>
where
    Src: Iterator<Item = char>,
{
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        match self.source.next() {
            Some(c) => match c {
                '<' => Some(Token {
                    kind: Kind::LeftArrow,
                    literal: c,
                }),
                '>' => Some(Token {
                    kind: Kind::RightArrow,
                    literal: c,
                }),
                '/' => Some(Token {
                    kind: Kind::Slash,
                    literal: c,
                }),
                '=' => Some(Token {
                    kind: Kind::Equal,
                    literal: c,
                }),
                '"' => Some(Token {
                    kind: Kind::Quote,
                    literal: c,
                }),
                _ => {
                    if c.is_whitespace() {
                        Some(Token {
                            kind: Kind::WhiteSpace,
                            literal: c,
                        })
                    } else {
                        Some(Token {
                            kind: Kind::Text,
                            literal: c,
                        })
                    }
                }
            },
            None => None,
        }
    }
}
