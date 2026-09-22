use crate::ast::Span;
use chumsky::DefaultExpected;
use chumsky::input::Input;
use chumsky::label::LabelError;
use chumsky::util::MaybeRef;
use gyokuto_lexer::{LexError, Token};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
#[error("{kind}")]
pub struct Error {
    pub span: Span,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ErrorKind {
    #[error(transparent)]
    Lex(#[from] LexError),
    #[error(transparent)]
    Literal(#[from] LiteralError),
    #[error("invalid tuple index")]
    InvalidTupleIndex,
    #[error("unexpected {found:?}, expected one of {expected:?}")]
    Syntax {
        expected: Vec<Expected>,
        found: Found,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    Token(Token),
    EndOfInput,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expected {
    Token(Token),
    Any,
    EndOfInput,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LiteralError {
    #[error("integer literal is too large")]
    IntegerTooLarge,
    #[error("literal is not valid UTF-8")]
    InvalidUtf8,
    #[error("invalid escape sequence")]
    InvalidEscape,
    #[error("malformed literal")]
    Malformed,
    #[error("not a literal token")]
    NotALiteral,
}

impl<'a, I: Input<'a, Token = Token, Span = Span>> chumsky::error::Error<'a, I> for Error {
    fn merge(mut self, other: Self) -> Self {
        if let (ErrorKind::Syntax { expected, .. }, ErrorKind::Syntax { expected: more, .. }) =
            (&mut self.kind, other.kind)
        {
            let more: Vec<_> = more.into_iter().filter(|e| !expected.contains(e)).collect();
            expected.extend(more);
        }
        self
    }
}

impl<'a, I: Input<'a, Token = Token, Span = Span>> LabelError<'a, I, DefaultExpected<'a, Token>>
    for Error
{
    fn expected_found<E: IntoIterator<Item = DefaultExpected<'a, Token>>>(
        expected: E,
        found: Option<MaybeRef<'a, Token>>,
        span: Span,
    ) -> Self {
        let expected = expected
            .into_iter()
            .map(|e| match e {
                DefaultExpected::Token(token) => Expected::Token(*token),
                DefaultExpected::Any => Expected::Any,
                DefaultExpected::EndOfInput => Expected::EndOfInput,
                _ => Expected::Other,
            })
            .collect();
        Error {
            span,
            kind: ErrorKind::Syntax {
                expected,
                found: found.map_or(Found::EndOfInput, |token| Found::Token(*token)),
            },
        }
    }
}
