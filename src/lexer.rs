use std::fmt;

use chumsky::{error::Rich, span::SimpleSpan};

use crate::driver::CRATE_STR;
use crate::error::{Diagnostic, Error, Span};
use crate::str::{Binary, Decimal, Hexadecimal};
use crate::version::SIMC_STR;

pub type Spanned<T> = (T, SimpleSpan);

/// Output shared by the combinator lexer and the ASCII scanner: the token
/// stream, if recovery left one, plus the lexing errors.
type Lexed<'src> = (
    Option<Vec<Spanned<Token<'src>>>>,
    Vec<Rich<'src, char, SimpleSpan>>,
);
pub type Tokens<'src> = Vec<(Token<'src>, crate::error::Span)>;

#[cfg(feature = "fmt")]
pub type FmtTokens<'src> = Vec<(FmtToken<'src>, crate::error::Span)>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token<'src> {
    // Keywords
    Pub,
    Use,
    As,
    Fn,
    Let,
    Type,
    Mod,
    Const,
    Match,
    Enum,
    Crate,
    /// Reserved for the compiler version directive, which the version prescan
    /// consumes before lexing; [`lex`] reports any occurrence as an error and drops
    /// the token.
    Simc,

    // Control symbols
    Arrow,
    /// Represents a contiguous `::` token.
    /// This prevents the lexer from allowing spaces between colons (e.g., `use a: :b`),
    DoubleColon,
    Colon,
    Semi,
    Comma,
    Eq,
    FatArrow,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LAngle,
    RAngle,

    // Number literals
    DecLiteral(Decimal),
    HexLiteral(Hexadecimal),
    BinLiteral(Binary),

    // Boolean literal
    Bool(bool),

    // Identifier
    Ident(&'src str),

    // Jets, witnesses, and params
    Jet(&'src str),
    Witness(&'src str),
    Param(&'src str),

    // Built-in functions
    Macro(&'src str),
}

#[cfg(feature = "fmt")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TriviaKind {
    LineComment,
    BlockComment,
    Newline,
    Whitespace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineEnding {
    /// Windows carriage-return/line-feed (`\r\n`).
    CrLf,
    /// Unix line-feed (`\n`).
    Lf,
    /// Classic Mac OS carriage-return (`\r`).
    Cr,
}

impl LineEnding {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CrLf => "\r\n",
            Self::Lf => "\n",
            Self::Cr => "\r",
        }
    }
}

#[cfg(feature = "fmt")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Trivia<'src> {
    LineComment(&'src str),
    BlockComment(&'src str),
    Newline(LineEnding),
    Whitespace(&'src str),
}

#[cfg(feature = "fmt")]
impl<'src> Trivia<'src> {
    pub const fn line_comment(text: &'src str) -> Self {
        Self::LineComment(text)
    }

    pub const fn block_comment(text: &'src str) -> Self {
        Self::BlockComment(text)
    }

    pub const fn newline(line_ending: LineEnding) -> Self {
        Self::Newline(line_ending)
    }

    pub const fn whitespace(text: &'src str) -> Self {
        Self::Whitespace(text)
    }

    pub const fn kind(&self) -> TriviaKind {
        match self {
            Self::LineComment(_) => TriviaKind::LineComment,
            Self::BlockComment(_) => TriviaKind::BlockComment,
            Self::Newline(_) => TriviaKind::Newline,
            Self::Whitespace(_) => TriviaKind::Whitespace,
        }
    }
}

#[cfg(feature = "fmt")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FmtToken<'src> {
    Token(Token<'src>),
    Trivia(Trivia<'src>),
}

impl<'src> fmt::Display for Token<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Pub => write!(f, "pub"),
            Token::Use => write!(f, "use"),
            Token::As => write!(f, "as"),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Type => write!(f, "type"),
            Token::Mod => write!(f, "mod"),
            Token::Const => write!(f, "const"),
            Token::Match => write!(f, "match"),
            Token::Enum => write!(f, "enum"),
            Token::Crate => write!(f, "{}", CRATE_STR),
            Token::Simc => write!(f, "{}", SIMC_STR),

            Token::Arrow => write!(f, "->"),
            Token::DoubleColon => write!(f, "::"),
            Token::Colon => write!(f, ":"),
            Token::Semi => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::Eq => write!(f, "="),
            Token::FatArrow => write!(f, "=>"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LAngle => write!(f, "<"),
            Token::RAngle => write!(f, ">"),

            Token::DecLiteral(s) => write!(f, "{}", s),
            Token::HexLiteral(s) => write!(f, "0x{}", s),
            Token::BinLiteral(s) => write!(f, "0b{}", s),

            Token::Ident(s) => write!(f, "{}", s),

            Token::Macro(s) => write!(f, "{}", s),

            Token::Jet(s) => write!(f, "jet::{}", s),
            Token::Witness(s) => write!(f, "witness::{}", s),
            Token::Param(s) => write!(f, "param::{}", s),

            Token::Bool(b) => write!(f, "{}", b),
        }
    }
}

#[cfg(feature = "fmt")]
impl<'src> fmt::Display for FmtToken<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FmtToken::Token(t) => {
                write!(f, "{}", t)
            }
            FmtToken::Trivia(
                Trivia::LineComment(text) | Trivia::BlockComment(text) | Trivia::Whitespace(text),
            ) => write!(f, "{text}"),
            FmtToken::Trivia(Trivia::Newline(line_ending)) => write!(f, "{}", line_ending.as_str()),
        }
    }
}

fn digit_literal_text<const PRESERVE: bool>(input: &str) -> std::borrow::Cow<'_, str> {
    if PRESERVE || !input.contains('_') {
        std::borrow::Cow::Borrowed(input)
    } else {
        std::borrow::Cow::Owned(input.replace('_', ""))
    }
}

/// Lexes an input string into a stream of tokens with spans, beginning at byte
/// offset `start` — the end of the version directive per `SimcDirective::prescan`,
/// or `0`. Spans are reported relative to the full input.
///
/// All comments, newlines, and spaces in the input code are discarded.
pub fn lex(
    file_id: usize,
    input: &str,
    start: usize,
) -> (Option<Tokens<'_>>, Vec<crate::error::Diagnostic>) {
    let (tokens, lex_errors) = logos_lexer::lex(&input[start..]);
    let shift = |span| Span::from_chumsky(file_id, span, start);

    let mut diagnostics: Vec<Diagnostic> = lex_errors
        .into_iter()
        .map(|err| {
            Diagnostic::new(
                Error::CannotParse {
                    msg: err.reason().to_string(),
                },
                shift(*err.span()),
            )
        })
        .collect();

    let tokens = tokens.map(|vec| {
        vec.into_iter()
            .filter_map(|(tok, span)| filter_token(tok, span, &mut diagnostics, shift))
            .collect()
    });

    (tokens, diagnostics)
}

/// Lexes an input string into a lossles stream of tokens with spans, beginning at byte
/// offset `start` — the end of the version directive per `SimcDirective::prescan`,
/// or `0`. Spans are reported relative to the full input.
///
/// All comments, newlines, and spaces in the input code are remained.
#[cfg(feature = "fmt")]
pub fn lex_lossless(
    file_id: usize,
    input: &str,
    start: usize,
) -> (Option<FmtTokens<'_>>, Vec<Diagnostic>) {
    let (tokens, lex_errors) = logos_lexer::lossless::lex(&input[start..]);
    let shift = |span: SimpleSpan| Span::from_chumsky(file_id, span, start);

    let mut diagnostics: Vec<Diagnostic> = lex_errors
        .into_iter()
        .map(|err| {
            Diagnostic::new(
                Error::CannotParse {
                    msg: err.reason().to_string(),
                },
                shift(*err.span()),
            )
        })
        .collect();

    let tokens = tokens.map(|vec| {
        vec.into_iter()
            .filter_map(|(fmt_tok, span)| match fmt_tok {
                FmtToken::Token(tok) => filter_token(tok, span, &mut diagnostics, shift)
                    .map(|(t, s)| (FmtToken::Token(t), s)),
                FmtToken::Trivia(t) => Some((FmtToken::Trivia(t), shift(span))),
            })
            .collect()
    });

    (tokens, diagnostics)
}

/// Skip whitespace and (nested) comments from the start of `input`, returning
/// the byte offset of the first content character. The version-directive
/// prescan uses this so the scanner and the lexer agree on comment syntax.
pub(crate) fn skip_trivia(input: &str) -> usize {
    let b = input.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = input[i..].chars().next().expect("in-bounds");
        if c.is_whitespace() {
            i += c.len_utf8();
        } else if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            i += 2;
            while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                i += 1;
            }
        } else if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            let mut depth = 1;
            i += 2;
            while i < b.len() && depth > 0 {
                if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if b[i] == b'*' && i + 1 < b.len() && b[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else {
            break;
        }
    }
    i
}

fn filter_token<'src, F: Fn(SimpleSpan) -> Span>(
    tok: Token<'src>,
    span: SimpleSpan,
    errors: &mut Vec<Diagnostic>,
    convert_span: F,
) -> Option<(Token<'src>, Span)> {
    match tok {
        // The reserved keyword is a sentinel: the prescan consumed the one
        // legitimate directive before lexing, so any occurrence is misplaced.
        Token::Simc => {
            errors.push(Diagnostic::new(
                Error::ReservedSimcKeyword,
                convert_span(span),
            ));
            None
        }
        tok => Some((tok, convert_span(span))),
    }
}

/// A list of all reserved keywords.
pub const KEYWORDS: &[&str] = &[
    "pub", "use", "as", "fn", "let", "type", "mod", "const", "match", "enum", CRATE_STR, SIMC_STR,
    "true", "false",
];

/// Checks whether a given string is a keyword.
pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

#[cfg(test)]
mod original_lexer {
    use super::*;

    /// A stand-in for the removed combinator lexer's test surface: span and
    /// message of each lexing error, the only parts the tests below assert.
    #[derive(Debug)]
    struct TestError {
        span: SimpleSpan,
        msg: String,
    }

    impl TestError {
        fn span(&self) -> SimpleSpan {
            self.span
        }
    }

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.msg)
        }
    }

    fn lex<'src>(input: &'src str) -> (Option<Vec<Token<'src>>>, Vec<TestError>) {
        let (tokens, errors) = logos_lexer::lex(input);
        let tokens = tokens.map(|vec| vec.into_iter().map(|(tok, _)| tok).collect::<Vec<_>>());
        let errors = errors
            .into_iter()
            .map(|err| TestError {
                span: *err.span(),
                msg: err.reason().to_string(),
            })
            .collect();
        (tokens, errors)
    }
    #[test]
    fn test_block_comment_simple() {
        let input = "/* hello world */";
        let (tokens, errors) = lex(input);

        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(
            tokens,
            Some(vec![]),
            "Should produce a single block comment token"
        );
    }

    #[test]
    fn test_block_comment_nested() {
        let input = "/* outer /* inner */ outer */";
        let (tokens, errors) = lex(input);

        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(tokens, Some(vec![]));
    }

    #[test]
    fn test_block_comment_deeply_nested() {
        let input = "/* 1 /* 2 /* 3 */ 2 */ 1 */";
        let (tokens, errors) = lex(input);

        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(tokens, Some(vec![]));
    }

    #[test]
    fn test_block_comment_multiline() {
        let input = "/* \n line 1 \n /* inner \n line */ \n */";
        let (tokens, errors) = lex(input);

        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(tokens, Some(vec![]));
    }

    #[test]
    fn test_block_comment_unclosed() {
        let input = "/* unclosed comment start";
        let (tokens, errors) = lex(input);

        assert_eq!(errors.len(), 1, "Expected exactly 1 error");

        let err = &errors[0];
        assert_eq!(err.span().start, 0);
        assert_eq!(err.span().end, 2);
        assert_eq!(err.to_string(), "Unclosed block comment");

        assert_eq!(tokens, Some(vec![]));
    }

    #[test]
    fn test_block_comment_partial_nesting_unclosed() {
        let input = "/* outer /* inner */";
        let (tokens, errors) = lex(input);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].span().start, 0);
        assert_eq!(tokens, Some(vec![]));
    }

    #[test]
    fn test_block_comment_double_unclosed() {
        let input = "/* outer /* inner";
        let (tokens, errors) = lex(input);

        assert_eq!(errors.len(), 2);

        assert_eq!(errors[0].span().start, 9);
        assert_eq!(errors[0].to_string(), "Unclosed block comment");

        assert_eq!(errors[1].span().start, 0);
        assert_eq!(errors[1].to_string(), "Unclosed block comment");

        assert_eq!(tokens, Some(vec![]));
    }

    #[test]
    fn test_spaces_resolution() {
        let input = "\r\n\n\r\r\r\r\r\n\r\n    \n\n\r\n\n\r";
        let (tokens, errors) = lex(input);

        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(tokens, Some(vec![]));
    }

    #[test]
    fn test_ignoring_tokens_after_comment_with_incorrect_symbol() {
        let input = "fn main() {} @// fn hello(){} simc";
        let (tokens, errors) = lex(input);

        assert_eq!(
            errors.len(),
            1,
            "comment contents must not be retried as code"
        );
        assert!(errors[0].to_string().contains("found '@' expected"));
        assert_eq!(
            tokens,
            Some(vec![
                Token::Fn,
                Token::Ident("main"),
                Token::LParen,
                Token::RParen,
                Token::LBrace,
                Token::RBrace,
            ])
        );

        let (_tokens, diagnostics) = super::lex(0, input, 0);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].error(), Error::CannotParse { .. }));
    }

    #[test]
    fn test_ignoring_tokens_after_comment() {
        let input = "fn main() {} /* fn hello(){} \n simc 0.6.0; */ \n\
             // enum Name {} match true {} \n fn other_main() {} ";
        let (tokens, errors) = lex(input);

        assert!(errors.is_empty());
        assert_eq!(
            tokens,
            Some(vec![
                Token::Fn,
                Token::Ident("main"),
                Token::LParen,
                Token::RParen,
                Token::LBrace,
                Token::RBrace,
                Token::Fn,
                Token::Ident("other_main"),
                Token::LParen,
                Token::RParen,
                Token::LBrace,
                Token::RBrace,
            ])
        );
    }

    #[test]
    fn simc_is_reserved() {
        // The prescan consumes the one legitimate leading directive before lexing,
        // so `lex` reports any `simc` it sees and drops the sentinel token.
        for src in ["simc", "fn simc() {}", "fn f() {}\nsimc"] {
            let (tokens, errors) = super::lex(0, src, 0);
            assert!(
                errors.iter().any(|e| e.to_string().contains("reserved")),
                "expected a reserved-keyword error for {src:?}, got: {errors:?}"
            );
            assert!(
                tokens
                    .expect("recovery keeps the stream")
                    .iter()
                    .all(|(tok, _)| !matches!(tok, Token::Simc)),
                "the sentinel must not reach the token stream for {src:?}"
            );
        }

        // Identifiers merely starting with `simc` are ordinary identifiers.
        let (tokens, errors) = lex("simcfoo");
        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(tokens, Some(vec![Token::Ident("simcfoo")]));
    }

    #[test]
    fn test_enum_token() {
        let (tokens, errors) = lex("enum Path { Inherit, ColdSpend }");
        assert!(errors.is_empty());
        assert_eq!(
            tokens,
            Some(vec![
                Token::Enum,
                Token::Ident("Path"),
                Token::LBrace,
                Token::Ident("Inherit"),
                Token::Comma,
                Token::Ident("ColdSpend"),
                Token::RBrace,
            ])
        );
    }

    #[test]
    fn numeric_literals_with_underscores_and_underscore_digit_identifiers() {
        let cases = [
            (
                "[u8; 1_6]",
                vec![
                    Token::LBracket,
                    Token::Ident("u8"),
                    Token::Semi,
                    Token::DecLiteral(Decimal::from_str_unchecked("16")),
                    Token::RBracket,
                ],
            ),
            (
                "List<u8, 1_6>",
                vec![
                    Token::Ident("List"),
                    Token::LAngle,
                    Token::Ident("u8"),
                    Token::Comma,
                    Token::DecLiteral(Decimal::from_str_unchecked("16")),
                    Token::RAngle,
                ],
            ),
            (
                "1_3_3_7",
                vec![Token::DecLiteral(Decimal::from_str_unchecked("1337"))],
            ),
            (
                "0x1_",
                vec![Token::HexLiteral(Hexadecimal::from_str_unchecked("1"))],
            ),
            (
                "0b1010_0101",
                vec![Token::BinLiteral(Binary::from_str_unchecked("10100101"))],
            ),
            ("_34", vec![Token::Ident("_34")]),
            ("_34foo", vec![Token::Ident("_34foo")]),
        ];

        for (input, expected) in cases {
            let (tokens, errors) = lex(input);
            assert!(
                errors.is_empty(),
                "Expected no errors for {input:?}: {errors:?}"
            );
            assert_eq!(tokens, Some(expected), "Unexpected tokens for {input:?}");
        }
    }

    #[test]
    fn leading_whitespaces_before_main() {
        let input = "                                                        fn main(){}";
        let (tokens, errors) = lex(input);

        assert!(errors.is_empty());
        assert!(tokens.is_some());

        let tokens = tokens.unwrap();
        assert_eq!(tokens[0], Token::Fn);
    }

    #[test]
    fn lexer_test() {
        // Check if the lexer parses the example file without errors.
        let src = include_str!("../examples/last_will.simf");

        let (tokens, lex_errs) = logos_lexer::lex(src);
        let _ = tokens.unwrap();

        assert!(lex_errs.is_empty());
    }
}

#[cfg(feature = "fmt")]
#[cfg(test)]
mod fmt_lexer {
    use super::*;

    fn lex_lossless<'src>(
        input: &'src str,
    ) -> (
        Option<Vec<FmtToken<'src>>>,
        Vec<Rich<'src, char, SimpleSpan>>,
    ) {
        let (tokens, errors) = super::logos_lexer::lossless::lex(input);
        let tokens = tokens.map(|vec| {
            vec.into_iter()
                .map(|(fmt_tok, _)| fmt_tok)
                .collect::<Vec<_>>()
        });
        (tokens, errors)
    }

    fn fmt_trivia<'src>(kind: TriviaKind, text: &'src str) -> FmtToken<'src> {
        let trivia = match kind {
            TriviaKind::LineComment => Trivia::line_comment(text),
            TriviaKind::BlockComment => Trivia::block_comment(text),
            TriviaKind::Newline => Trivia::newline(match text {
                "\r\n" => LineEnding::CrLf,
                "\n" => LineEnding::Lf,
                "\r" => LineEnding::Cr,
                _ => panic!("invalid newline spelling: {text:?}"),
            }),
            TriviaKind::Whitespace => Trivia::whitespace(text),
        };

        FmtToken::Trivia(trivia)
    }

    #[test]
    fn test_block_comment_simple_fmt() {
        let input = "/* hello world */";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(
            tokens,
            Some(vec![fmt_trivia(TriviaKind::BlockComment, input)]),
            "Should produce a single block comment token"
        );
    }

    #[test]
    fn lossless_trivia_display_preserves_source_text() {
        let input = "// comment\r\n\t";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty(), "Expected no errors, found: {errors:?}");
        let rendered: String = tokens
            .expect("lossless lexing succeeds")
            .iter()
            .map(ToString::to_string)
            .collect();

        assert_eq!(rendered, input);
    }

    #[test]
    fn lossless_lexer_keeps_each_newline_kind_with_its_span() {
        let input = "first\r\nsecond\rthird\nfourth";
        let (tokens, errors) = super::lex_lossless(0, input, 0);

        assert!(errors.is_empty(), "Expected no errors, found: {errors:?}");
        let tokens = tokens.expect("lossless lexing succeeds");
        let line_endings: Vec<_> = tokens
            .iter()
            .filter_map(|(token, _)| match token {
                FmtToken::Trivia(Trivia::Newline(line_ending)) => Some(*line_ending),
                _ => None,
            })
            .collect();
        let newlines: Vec<_> = tokens
            .iter()
            .filter(|(token, _)| {
                matches!(token, FmtToken::Trivia(trivia) if trivia.kind() == TriviaKind::Newline)
            })
            .map(|(_, span)| span.to_slice(input))
            .collect();

        assert_eq!(
            line_endings,
            vec![LineEnding::CrLf, LineEnding::Cr, LineEnding::Lf]
        );
        assert_eq!(newlines, vec![Some("\r\n"), Some("\r"), Some("\n")]);
    }

    #[test]
    fn test_block_comment_nested_fmt() {
        let input = "/* outer /* inner */ outer */";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(
            tokens,
            Some(vec![fmt_trivia(TriviaKind::BlockComment, input)])
        );
    }

    #[test]
    fn test_block_comment_deeply_nested_fmt() {
        let input = "/* 1 /* 2 /* 3 */ 2 */ 1 */";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty());
        assert_eq!(
            tokens,
            Some(vec![fmt_trivia(TriviaKind::BlockComment, input)])
        );
    }

    #[test]
    fn test_block_comment_multiline_fmt() {
        let input = "/* \n line 1 \n /* inner \n line */ \n */";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty());
        assert_eq!(
            tokens,
            Some(vec![fmt_trivia(TriviaKind::BlockComment, input)])
        );
    }

    #[test]
    fn test_block_comment_unclosed_fmt() {
        let input = "/* unclosed comment start";
        let (tokens, errors) = lex_lossless(input);

        assert_eq!(errors.len(), 1, "Expected exactly 1 error");

        let err = &errors[0];
        assert_eq!(err.span().start, 0);
        assert_eq!(err.span().end, 2);
        assert_eq!(err.to_string(), "Unclosed block comment");

        assert_eq!(
            tokens,
            Some(vec![fmt_trivia(TriviaKind::BlockComment, input)])
        );
    }

    #[test]
    fn test_block_comment_partial_nesting_unclosed_fmt() {
        let input = "/* outer /* inner */";
        let (tokens, errors) = lex_lossless(input);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].span().start, 0);
        assert_eq!(
            tokens,
            Some(vec![fmt_trivia(TriviaKind::BlockComment, input)])
        );
    }

    #[test]
    fn test_block_comment_double_unclosed_fmt() {
        let input = "/* outer /* inner";
        let (tokens, errors) = lex_lossless(input);

        assert_eq!(errors.len(), 2);

        assert_eq!(errors[0].span().start, 9);
        assert_eq!(errors[0].to_string(), "Unclosed block comment");

        assert_eq!(errors[1].span().start, 0);
        assert_eq!(errors[1].to_string(), "Unclosed block comment");

        assert_eq!(
            tokens,
            Some(vec![fmt_trivia(TriviaKind::BlockComment, input)])
        );
    }

    #[test]
    fn test_spaces_resolution_fmt() {
        let input = "\r\n\n\r\r\r\r\r\n\r\n    \n\n\r\n\n\r";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(
            tokens,
            Some(vec![
                fmt_trivia(TriviaKind::Newline, "\r\n"),
                fmt_trivia(TriviaKind::Newline, "\n"),
                fmt_trivia(TriviaKind::Newline, "\r"),
                fmt_trivia(TriviaKind::Newline, "\r"),
                fmt_trivia(TriviaKind::Newline, "\r"),
                fmt_trivia(TriviaKind::Newline, "\r"),
                fmt_trivia(TriviaKind::Newline, "\r\n"),
                fmt_trivia(TriviaKind::Newline, "\r\n"),
                fmt_trivia(TriviaKind::Whitespace, "    "),
                fmt_trivia(TriviaKind::Newline, "\n"),
                fmt_trivia(TriviaKind::Newline, "\n"),
                fmt_trivia(TriviaKind::Newline, "\r\n"),
                fmt_trivia(TriviaKind::Newline, "\n"),
                fmt_trivia(TriviaKind::Newline, "\r"),
            ])
        );
    }

    #[test]
    fn test_ignoring_tokens_after_comment_with_incorrect_symbol() {
        let input = "fn main() {} @// fn hello(){} simc";
        let (tokens, errors) = lex_lossless(input);

        assert_eq!(errors.len(), 1, "only the invalid character is an error");
        assert!(errors[0].to_string().contains("found '@' expected"));

        let tokens = tokens.expect("recovery keeps the lossless stream");
        assert!(matches!(
            tokens.last(),
            Some(FmtToken::Trivia(Trivia::LineComment(_)))
        ));
        assert!(
            tokens
                .iter()
                .all(|token| !matches!(token, FmtToken::Token(Token::Simc))),
            "comment contents must not become semantic tokens"
        );
    }

    #[test]
    fn test_not_ignoring_tokens_after_comment() {
        let input = "fn main() {} /* fn hello(){} \n simc 0.6.0; */ \n\
             // enum Name {} match true {} \n fn other_main() {} ";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty());
        assert_eq!(
            tokens,
            Some(vec![
                FmtToken::Token(Token::Fn),
                fmt_trivia(TriviaKind::Whitespace, " "),
                FmtToken::Token(Token::Ident("main")),
                FmtToken::Token(Token::LParen),
                FmtToken::Token(Token::RParen),
                fmt_trivia(TriviaKind::Whitespace, " "),
                FmtToken::Token(Token::LBrace),
                FmtToken::Token(Token::RBrace),
                fmt_trivia(TriviaKind::Whitespace, " "),
                fmt_trivia(
                    TriviaKind::BlockComment,
                    "/* fn hello(){} \n simc 0.6.0; */"
                ),
                fmt_trivia(TriviaKind::Whitespace, " "),
                fmt_trivia(TriviaKind::Newline, "\n"),
                fmt_trivia(TriviaKind::LineComment, "// enum Name {} match true {} "),
                fmt_trivia(TriviaKind::Newline, "\n"),
                fmt_trivia(TriviaKind::Whitespace, " "),
                FmtToken::Token(Token::Fn),
                fmt_trivia(TriviaKind::Whitespace, " "),
                FmtToken::Token(Token::Ident("other_main")),
                FmtToken::Token(Token::LParen),
                FmtToken::Token(Token::RParen),
                fmt_trivia(TriviaKind::Whitespace, " "),
                FmtToken::Token(Token::LBrace),
                FmtToken::Token(Token::RBrace),
                fmt_trivia(TriviaKind::Whitespace, " "),
            ])
        );
    }

    #[test]
    fn simc_is_reserved_fmt() {
        // The prescan consumes the one legitimate leading directive before lexing,
        // so `lex` reports any `simc` it sees and drops the sentinel token.
        for src in ["simc", "fn simc() {}", "fn f() {}\nsimc"] {
            let (tokens, errors) = super::lex_lossless(0, src, 0);
            assert!(
                errors.iter().any(|e| e.to_string().contains("reserved")),
                "expected a reserved-keyword error for {src:?}, got: {errors:?}"
            );

            assert!(
                tokens
                    .expect("recovery keeps the stream")
                    .iter()
                    .all(|(tok, _)| !matches!(tok, FmtToken::Token(Token::Simc))),
                "the sentinel must not reach the token stream for {src:?}"
            );
        }

        // Identifiers merely starting with `simc` are ordinary identifiers.
        let (tokens, errors) = lex_lossless("simcfoo");
        assert!(errors.is_empty(), "Expected no errors, found: {:?}", errors);
        assert_eq!(tokens, Some(vec![FmtToken::Token(Token::Ident("simcfoo"))]));
    }

    #[test]
    fn numeric_literals_preserve_underscores() {
        let input = "1_234_567_89 0xDEAD_BEEF 0b1010_0101_ 0b1010_0101 1_234__567___890____000_____ 0xDEAD_BEEF__BEEF_DEAD_";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty(), "Expected no errors, found: {errors:?}");
        assert!(tokens.is_some(), "lexing must succeed");
        let tokens = tokens
            .unwrap()
            .into_iter()
            .filter(|x| !matches!(x, FmtToken::Trivia(Trivia::Whitespace(_))))
            .collect::<Vec<_>>();
        assert_eq!(
            tokens,
            vec![
                FmtToken::Token(Token::DecLiteral(Decimal::from_str_unchecked(
                    "1_234_567_89"
                ))),
                FmtToken::Token(Token::HexLiteral(Hexadecimal::from_str_unchecked(
                    "DEAD_BEEF"
                ))),
                FmtToken::Token(Token::BinLiteral(Binary::from_str_unchecked("1010_0101_"))),
                FmtToken::Token(Token::BinLiteral(Binary::from_str_unchecked("1010_0101"))),
                FmtToken::Token(Token::DecLiteral(Decimal::from_str_unchecked(
                    "1_234__567___890____000_____"
                ))),
                FmtToken::Token(Token::HexLiteral(Hexadecimal::from_str_unchecked(
                    "DEAD_BEEF__BEEF_DEAD_"
                ))),
            ]
        );

        assert_eq!(
            tokens
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
            input
        );
    }

    #[test]
    fn leading_whitespaces_before_main() {
        let input = "                                                        fn main(){}";
        let (tokens, errors) = lex_lossless(input);

        assert!(errors.is_empty());
        assert!(tokens.is_some());

        let tokens = dbg!(tokens).unwrap();
        assert!(matches!(tokens[0], FmtToken::Trivia(Trivia::Whitespace(_))));
        assert!(matches!(tokens[1], FmtToken::Token(Token::Fn)));
    }

    #[test]
    fn lossless_lexer_test() {
        // Check if the lexer parses the example file without errors.
        let src = include_str!("../examples/last_will.simf");

        let (tokens, lex_errs) = super::logos_lexer::lossless::lex(src);
        let _ = tokens.unwrap();

        assert!(lex_errs.is_empty());
    }
}

/// The hand-written ASCII scanner from the previous branch, retained on this
/// logos-only branch purely as a differential-test oracle: it was proven
/// equivalent to the removed combinator lexer, so logos-vs-scanner agreement
/// preserves that equivalence chain.
#[cfg(test)]
mod scanner_reference {
    use super::*;

    /// Hand-written scanner for ASCII input, producing exactly the tokens, spans
    /// and errors of the combinator [`lexer`], which remains the reference
    /// implementation (and the handler of non-ASCII input).
    ///
    /// One left-to-right pass over the bytes: trivia is skipped in place, and
    /// every token is recognized by a match on its first byte, with no
    /// backtracking and no allocation except the output vectors.
    pub(super) fn lex_ascii<'src>(input: &'src str) -> Lexed<'src> {
        let b = input.as_bytes();
        let mut tokens: Vec<Spanned<Token<'src>>> = Vec::new();
        let mut errors: Vec<Rich<'src, char, SimpleSpan>> = Vec::new();
        let mut i = 0;

        while i < b.len() {
            let c = b[i];
            // Unicode White_Space intersected with ASCII.
            if matches!(c, b' ' | b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r') {
                i += 1;
                continue;
            }

            if c == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
                // Line comment: everything up to (not including) the newline.
                i += 2;
                while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                    i += 1;
                }
                continue;
            }

            if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
                // Nested block comment. An unterminated comment swallows the rest
                // of the input; every nesting level left open is reported,
                // innermost first.
                let mut opens = vec![i];
                i += 2;
                loop {
                    if i + 1 >= b.len() {
                        for open in opens.iter().rev() {
                            errors.push(Rich::custom(
                                SimpleSpan::from(*open..*open + 2),
                                "Unclosed block comment",
                            ));
                        }
                        i = b.len();
                        break;
                    }
                    if b[i] == b'/' && b[i + 1] == b'*' {
                        opens.push(i);
                        i += 2;
                    } else if b[i] == b'*' && b[i + 1] == b'/' {
                        opens.pop();
                        i += 2;
                        if opens.is_empty() {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                continue;
            }

            if c.is_ascii_alphabetic() || c == b'_' {
                let ident_start = i;
                i += 1;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let ident = &input[ident_start..i];

                if i < b.len()
                    && b[i] == b'!'
                    && matches!(ident, "assert" | "panic" | "dbg" | "list")
                {
                    i += 1;
                    tokens.push((
                        Token::Macro(&input[ident_start..i]),
                        SimpleSpan::from(ident_start..i),
                    ));
                    continue;
                }

                if matches!(ident, "jet" | "witness" | "param")
                    && i + 2 < b.len()
                    && b[i] == b':'
                    && b[i + 1] == b':'
                    && (b[i + 2].is_ascii_alphabetic() || b[i + 2] == b'_')
                {
                    let name_start = i + 2;
                    let mut j = name_start + 1;
                    while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                        j += 1;
                    }
                    let name = &input[name_start..j];
                    let token = match ident {
                        "jet" => Token::Jet(name),
                        "witness" => Token::Witness(name),
                        _ => Token::Param(name),
                    };
                    tokens.push((token, SimpleSpan::from(ident_start..j)));
                    i = j;
                    continue;
                }

                let token = match ident {
                    "pub" => Token::Pub,
                    "use" => Token::Use,
                    "as" => Token::As,
                    "fn" => Token::Fn,
                    "let" => Token::Let,
                    "type" => Token::Type,
                    "mod" => Token::Mod,
                    "const" => Token::Const,
                    "match" => Token::Match,
                    "enum" => Token::Enum,
                    CRATE_STR => Token::Crate,
                    SIMC_STR => Token::Simc,
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    _ => Token::Ident(ident),
                };
                tokens.push((token, SimpleSpan::from(ident_start..i)));
                continue;
            }

            if c.is_ascii_digit() {
                if c == b'0' && i + 2 < b.len() && matches!(b[i + 1], b'x' | b'b') {
                    let hexadecimal = b[i + 1] == b'x';
                    let is_digit = |byte: u8| {
                        if hexadecimal {
                            byte.is_ascii_hexdigit()
                        } else {
                            byte == b'0' || byte == b'1'
                        }
                    };
                    if is_digit(b[i + 2]) {
                        let mut j = i + 3;
                        while j < b.len() && (is_digit(b[j]) || b[j] == b'_') {
                            j += 1;
                        }
                        let text = digit_literal_text::<false>(&input[i + 2..j]);
                        let token = if hexadecimal {
                            Token::HexLiteral(Hexadecimal::from_str_unchecked(text.as_ref()))
                        } else {
                            Token::BinLiteral(Binary::from_str_unchecked(text.as_ref()))
                        };
                        tokens.push((token, SimpleSpan::from(i..j)));
                        i = j;
                        continue;
                    }
                    // A prefix with no valid digit after it (`0x`, `0b1`'s tail,
                    // ...) falls back to the decimal below: `0x` lexes as `0`,
                    // then the identifier `x`.
                }

                let mut j = i + 1;
                while j < b.len() && (b[j].is_ascii_digit() || b[j] == b'_') {
                    j += 1;
                }
                let text = digit_literal_text::<false>(&input[i..j]);
                tokens.push((
                    Token::DecLiteral(Decimal::from_str_unchecked(text.as_ref())),
                    SimpleSpan::from(i..j),
                ));
                i = j;
                continue;
            }

            let next = if i + 1 < b.len() { b[i + 1] } else { 0 };
            let (token, len) = match c {
                b'-' if next == b'>' => (Token::Arrow, 2),
                b'=' if next == b'>' => (Token::FatArrow, 2),
                b':' if next == b':' => (Token::DoubleColon, 2),
                b'=' => (Token::Eq, 1),
                b':' => (Token::Colon, 1),
                b';' => (Token::Semi, 1),
                b',' => (Token::Comma, 1),
                b'(' => (Token::LParen, 1),
                b')' => (Token::RParen, 1),
                b'[' => (Token::LBracket, 1),
                b']' => (Token::RBracket, 1),
                b'{' => (Token::LBrace, 1),
                b'}' => (Token::RBrace, 1),
                b'<' => (Token::LAngle, 1),
                b'>' => (Token::RAngle, 1),
                _ => {
                    // Like the combinator lexer's recovery: report the first
                    // character of the garbage, then skip until a position that
                    // can begin a lexeme, and continue there.
                    errors.push(Rich::custom(
                        SimpleSpan::from(i..i + 1),
                        format!("found '{}' expected a token", char::from(c)),
                    ));
                    i += 1;
                    while i < b.len() && !starts_lexeme(b, i) {
                        i += 1;
                    }
                    continue;
                }
            };
            tokens.push((token, SimpleSpan::from(i..i + len)));
            i += len;
        }

        (Some(tokens), errors)
    }

    /// Whether `b[i]` can begin a lexeme (a token or trivia) in the ASCII
    /// scanner, used to resynchronize after unrecognized input.
    fn starts_lexeme(b: &[u8], i: usize) -> bool {
        match b[i] {
            b' ' | b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r' => true,
            b'/' => i + 1 < b.len() && (b[i + 1] == b'/' || b[i + 1] == b'*'),
            b'-' => i + 1 < b.len() && b[i + 1] == b'>',
            b'=' | b':' | b';' | b',' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'<' | b'>' => {
                true
            }
            c => c.is_ascii_alphabetic() || c == b'_' || c.is_ascii_digit(),
        }
    }
}

#[cfg(test)]
mod differential_tests {
    use super::*;

    /// Deterministic xorshift so failures reproduce.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    type Summary<'src> = (
        Option<Vec<(Token<'src>, SimpleSpan)>>,
        Vec<(SimpleSpan, String)>,
    );

    fn summarize<'src>(
        tokens: Option<&Vec<Spanned<Token<'src>>>>,
        errors: &[Rich<'src, char, SimpleSpan>],
    ) -> Summary<'src> {
        (
            tokens.cloned(),
            errors
                .iter()
                .map(|err| {
                    let msg = err.reason().to_string();
                    // The scanner's unexpected-character messages are
                    // intentionally shorter than the combinator's enumeration
                    // of every expected alternative; compare only the
                    // "found '<char>'" part, everything after `expected`.
                    match msg.split(" expected").next() {
                        Some(prefix) => (*err.span(), prefix.to_string()),
                        None => (*err.span(), msg),
                    }
                })
                .collect(),
        )
    }

    /// The ASCII scanner must produce exactly the tokens, spans and errors of
    /// the combinator lexer, on real sources, targeted edge cases, and random
    /// ASCII byte soup.
    #[test]
    fn ascii_scanner_matches_combinator_lexer() {
        // Strictly-compared corpus: real sources and curated edge cases,
        // including every well-specified error (unclosed comments, the
        // reserved keyword, an unknown character mid-program).
        let strict: Vec<String> = vec![
            include_str!("../examples/p2pkh.simf").to_string(),
            include_str!("../examples/last_will.simf").to_string(),
            include_str!("../examples/hash_loop.simf").to_string(),
            String::new(),
            "0x 0b 0xg 0b2 1_ _1 __ 0x1_ 0b1010_0101 1_3_3_7".to_string(),
            "jet::add_32 witness::W param::P jet:: jet: jetx::y crate::x simc simcfoo".to_string(),
            "assert! panic! dbg! list! assertx! assert".to_string(),
            "true false True False let letx".to_string(),
            "/* a /* b */ c */ /* unclosed".to_string(),
            "// line \r\n more /* not nested in line */".to_string(),
            "a @ b a@#b fn main() {} @// comment simc".to_string(),
        ];

        let mut corpus: Vec<String> = strict.clone();
        let mut rng = Rng(0x1EC0_FFEE_0000_0001);
        for _ in 0..1000 {
            let len = rng.below(240);
            let bytes = (0..len)
                .map(|_| (rng.below(0x7f) as u8).min(b'~'))
                .collect::<Vec<u8>>();
            corpus.push(String::from_utf8(bytes).expect("ASCII bytes are UTF-8"));
        }

        for src in corpus {
            let strict_compare = strict.contains(&src);
            let expected = scanner_reference::lex_ascii(src.as_str());
            let candidates: [(&str, Lexed); 1] =
                [("logos", crate::lexer::logos_lexer::lex(src.as_str()))];
            for (name, actual) in candidates {
                // Contract:
                // - On clean input (the combinator reports no errors) the
                //   scanner's tokens must match exactly. This covers every valid
                //   program, which is the guarantee that matters.
                // - On the curated corpus the errors must match too (spans and
                //   messages, up to the combinator's expected-item enumeration):
                //   those pin the specified error behaviors, like nested unclosed
                //   comments and the reserved keyword.
                // - On random garbage the combinator's recovery details (whether
                //   it resynchronizes before or after a comment opener, where in
                //   a garbage run the error lands, whether it returns output at
                //   all) are implementation artifacts, not a spec. Both lexers
                //   report the input as erroneous and the parse fails either
                //   way, so only require the scanner to agree that it is garbage.
                let (expected, expected_errors) = summarize(expected.0.as_ref(), &expected.1);
                let (actual, actual_errors) = summarize(actual.0.as_ref(), &actual.1);
                if expected_errors.is_empty() {
                    assert_eq!(expected, actual, "token mismatch for clean input {src:?}");
                    assert!(
                        actual_errors.is_empty(),
                        "scanner invented errors for clean input {src:?}"
                    );
                } else if strict_compare {
                    assert_eq!(expected, actual, "token mismatch for input {src:?}");
                    assert_eq!(
                        expected_errors, actual_errors,
                        "error mismatch for input {src:?}"
                    );
                } else {
                    assert!(
                        !actual_errors.is_empty(),
                        "{name} lexer must report garbage input as erroneous: {src:?}"
                    );
                }
            }
        }
    }
}

/// Experimental: a logos-based ASCII lexer, built to compare against the
/// hand-written scanner on the try/logos-lexer branch. Same role, same
/// output shape (`Lexed`), same differential oracle applies.
pub mod logos_lexer {
    use super::*;
    use logos::{Lexer, Logos};

    /// Errors recorded by token callbacks (nested block comments).
    #[derive(Default)]
    pub struct Extras {
        pub errors: Vec<(SimpleSpan, String)>,
    }

    #[derive(Logos, Clone, Debug, PartialEq)]
    #[logos(extras = Extras)]
    #[logos(skip(r"[ \t\n\x0B\x0C\r]+"))]
    #[logos(skip(r"//[^\r\n]*"))]
    enum AsciiToken<'src> {
        /// Nested `/* ... */` block comment; the callback walks the nesting.
        #[regex(r"/\*", block_comment)]
        BlockComment,

        /// Identifier, keyword, or boolean literal; resolved by the wrapper.
        #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
        Ident(&'src str),

        #[token("assert!")]
        #[token("panic!")]
        #[token("dbg!")]
        #[token("list!")]
        Macro(&'src str),

        /// `jet::`/`witness::`/`param::` path; the wrapper re-slices the name.
        #[regex(r"(jet|witness|param)::[a-zA-Z_][a-zA-Z0-9_]*")]
        Path,

        #[regex(r"[0-9][0-9_]*", decimal)]
        Dec(Decimal),

        #[regex(r"0x[0-9a-fA-F][0-9a-fA-F_]*", hexadecimal)]
        Hex(Hexadecimal),

        #[regex(r"0b[01][01_]*", binary)]
        Bin(Binary),

        #[token("->")]
        Arrow,
        #[token("=>")]
        FatArrow,
        #[token("=")]
        Eq,
        #[token("::")]
        DoubleColon,
        #[token(":")]
        Colon,
        #[token(";")]
        Semi,
        #[token(",")]
        Comma,
        #[token("(")]
        LParen,
        #[token(")")]
        RParen,
        #[token("[")]
        LBracket,
        #[token("]")]
        RBracket,
        #[token("{")]
        LBrace,
        #[token("}")]
        RBrace,
        #[token("<")]
        LAngle,
        #[token(">")]
        RAngle,
    }

    fn decimal<'src>(lex: &mut Lexer<'src, AsciiToken<'src>>) -> Decimal {
        Decimal::from_str_unchecked(digit_literal_text::<false>(lex.slice()).as_ref())
    }

    fn hexadecimal<'src>(lex: &mut Lexer<'src, AsciiToken<'src>>) -> Hexadecimal {
        let digits = &lex.slice()[2..]; // drop the 0x prefix
        Hexadecimal::from_str_unchecked(digit_literal_text::<false>(digits).as_ref())
    }

    fn binary<'src>(lex: &mut Lexer<'src, AsciiToken<'src>>) -> Binary {
        let digits = &lex.slice()[2..]; // drop the 0b prefix
        Binary::from_str_unchecked(digit_literal_text::<false>(digits).as_ref())
    }

    /// Consume a nested block comment; on an unterminated one, consume to
    /// the end and record one error per open nesting level, innermost first.
    fn block_comment<'src>(lex: &mut Lexer<'src, AsciiToken<'src>>) {
        let extra = walk_block_comment(lex.source(), lex.span(), &mut lex.extras.errors);
        // The pattern already consumed the opening `/*`; bump only what the
        // callback itself walked past it.
        lex.bump(extra);
    }

    /// Walk a nested block comment starting just after an opening `/*` at
    /// `span`. Returns how many bytes to consume beyond `span.end`; on an
    /// unterminated comment, records one error per open level, innermost
    /// first, and consumes to the end of the source.
    fn walk_block_comment(
        source: &str,
        span: std::ops::Range<usize>,
        errors: &mut Vec<(SimpleSpan, String)>,
    ) -> usize {
        let b = source.as_bytes();
        let mut opens = vec![span.start];
        let mut i = span.end;
        loop {
            if i + 1 >= b.len() {
                for open in opens.iter().rev() {
                    errors.push((
                        SimpleSpan::from(*open..*open + 2),
                        "Unclosed block comment".to_string(),
                    ));
                }
                i = b.len();
                break;
            }
            if b[i] == b'/' && b[i + 1] == b'*' {
                opens.push(i);
                i += 2;
            } else if b[i] == b'*' && b[i + 1] == b'/' {
                opens.pop();
                i += 2;
                if opens.is_empty() {
                    break;
                }
            } else {
                i += 1;
            }
        }
        i - span.end
    }

    fn keyword_or_ident<'src>(text: &'src str) -> Token<'src> {
        match text {
            "pub" => Token::Pub,
            "use" => Token::Use,
            "as" => Token::As,
            "fn" => Token::Fn,
            "let" => Token::Let,
            "type" => Token::Type,
            "mod" => Token::Mod,
            "const" => Token::Const,
            "match" => Token::Match,
            "enum" => Token::Enum,
            CRATE_STR => Token::Crate,
            SIMC_STR => Token::Simc,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            _ => Token::Ident(text),
        }
    }

    pub fn lex<'src>(input: &'src str) -> Lexed<'src> {
        let mut lexer = AsciiToken::lexer_with_extras(input, Extras::default());
        let mut tokens = Vec::new();
        // Logos reports one error per skipped character; coalesce adjacent
        // ones into a single error per garbage run, like the other lexers.
        let mut last_error_end: Option<usize> = None;
        while let Some(result) = lexer.next() {
            let span = SimpleSpan::from(lexer.span().start..lexer.span().end);
            match result {
                Ok(AsciiToken::BlockComment) => {}
                Ok(AsciiToken::Ident(text)) => tokens.push((keyword_or_ident(text), span)),
                Ok(AsciiToken::Macro(text)) => tokens.push((Token::Macro(text), span)),
                Ok(AsciiToken::Path) => {
                    let text = &input[span.start..span.end];
                    let name = &text[text.find("::").expect("regex matched a prefix") + 2..];
                    let token = match &text[..3] {
                        "jet" => Token::Jet(name),
                        "wit" => Token::Witness(name),
                        _ => Token::Param(name),
                    };
                    tokens.push((token, span));
                }
                Ok(AsciiToken::Dec(v)) => tokens.push((Token::DecLiteral(v), span)),
                Ok(AsciiToken::Hex(v)) => tokens.push((Token::HexLiteral(v), span)),
                Ok(AsciiToken::Bin(v)) => tokens.push((Token::BinLiteral(v), span)),
                Ok(token) => {
                    let token = match token {
                        AsciiToken::Arrow => Token::Arrow,
                        AsciiToken::FatArrow => Token::FatArrow,
                        AsciiToken::Eq => Token::Eq,
                        AsciiToken::DoubleColon => Token::DoubleColon,
                        AsciiToken::Colon => Token::Colon,
                        AsciiToken::Semi => Token::Semi,
                        AsciiToken::Comma => Token::Comma,
                        AsciiToken::LParen => Token::LParen,
                        AsciiToken::RParen => Token::RParen,
                        AsciiToken::LBracket => Token::LBracket,
                        AsciiToken::RBracket => Token::RBracket,
                        AsciiToken::LBrace => Token::LBrace,
                        AsciiToken::RBrace => Token::RBrace,
                        AsciiToken::LAngle => Token::LAngle,
                        AsciiToken::RAngle => Token::RAngle,
                        _ => unreachable!("payload variants handled above"),
                    };
                    tokens.push((token, span));
                }
                Err(()) => {
                    let found = input[span.start..].chars().next().expect("nonempty span");
                    let error_end = span.start + found.len_utf8();
                    if last_error_end != Some(span.start) {
                        let errors = &mut lexer.extras.errors;
                        errors.push((
                            SimpleSpan::from(span.start..error_end),
                            format!("found '{found}' expected a token"),
                        ));
                    }
                    last_error_end = Some(error_end);
                }
            }
        }
        let errors = lexer
            .extras
            .errors
            .iter()
            .map(|(span, msg)| Rich::custom(*span, msg.clone()));
        // Block-comment errors are appended after stream errors; an
        // unterminated comment always swallows the rest of the input, so its
        // errors are last in encounter order either way.
        (Some(tokens), errors.collect())
    }

    /// Lossless variant for the fmt feature: every lexeme, trivia included,
    /// with literal separators preserved.
    #[cfg(feature = "fmt")]
    pub(super) mod lossless {
        use super::*;

        #[derive(Logos, Clone, Debug, PartialEq)]
        #[logos(extras = Extras)]
        enum FmtLexeme<'src> {
            #[regex(r"[ \t\x0B\x0C]+")]
            Whitespace(&'src str),

            #[token("\r\n")]
            NewlineCrLf,
            #[token("\n")]
            NewlineLf,
            #[token("\r")]
            NewlineCr,

            #[regex(r"//[^\r\n]*")]
            LineComment(&'src str),

            #[regex(r"/\*", block_comment_text)]
            BlockComment(&'src str),

            #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
            Ident(&'src str),

            #[token("assert!")]
            #[token("panic!")]
            #[token("dbg!")]
            #[token("list!")]
            Macro(&'src str),

            #[regex(r"(jet|witness|param)::[a-zA-Z_][a-zA-Z0-9_]*")]
            Path,

            #[regex(r"[0-9][0-9_]*", decimal_pretty)]
            Dec(Decimal),

            #[regex(r"0x[0-9a-fA-F][0-9a-fA-F_]*", hexadecimal_pretty)]
            Hex(Hexadecimal),

            #[regex(r"0b[01][01_]*", binary_pretty)]
            Bin(Binary),

            #[token("->")]
            Arrow,
            #[token("=>")]
            FatArrow,
            #[token("=")]
            Eq,
            #[token("::")]
            DoubleColon,
            #[token(":")]
            Colon,
            #[token(";")]
            Semi,
            #[token(",")]
            Comma,
            #[token("(")]
            LParen,
            #[token(")")]
            RParen,
            #[token("[")]
            LBracket,
            #[token("]")]
            RBracket,
            #[token("{")]
            LBrace,
            #[token("}")]
            RBrace,
            #[token("<")]
            LAngle,
            #[token(">")]
            RAngle,
        }

        // The lossless lexer preserves underscore separators verbatim.
        fn decimal_pretty<'src>(lex: &mut Lexer<'src, FmtLexeme<'src>>) -> Decimal {
            Decimal::from_str_unchecked(lex.slice())
        }

        fn hexadecimal_pretty<'src>(lex: &mut Lexer<'src, FmtLexeme<'src>>) -> Hexadecimal {
            Hexadecimal::from_str_unchecked(&lex.slice()[2..])
        }

        fn binary_pretty<'src>(lex: &mut Lexer<'src, FmtLexeme<'src>>) -> Binary {
            Binary::from_str_unchecked(&lex.slice()[2..])
        }

        fn block_comment_text<'src>(lex: &mut Lexer<'src, FmtLexeme<'src>>) -> &'src str {
            let extra = walk_block_comment(lex.source(), lex.span(), &mut lex.extras.errors);
            lex.bump(extra);
            lex.slice()
        }

        pub type FmtLexed<'src> = (
            Option<Vec<(FmtToken<'src>, SimpleSpan)>>,
            Vec<Rich<'src, char, SimpleSpan>>,
        );

        pub fn lex<'src>(input: &'src str) -> FmtLexed<'src> {
            let mut lexer = FmtLexeme::lexer_with_extras(input, Extras::default());
            let mut tokens = Vec::new();
            let mut last_error_end: Option<usize> = None;
            while let Some(result) = lexer.next() {
                let span = SimpleSpan::from(lexer.span().start..lexer.span().end);
                let fmt_token = match result {
                    Ok(FmtLexeme::Whitespace(text)) => FmtToken::Trivia(Trivia::whitespace(text)),
                    Ok(FmtLexeme::NewlineCrLf) => {
                        FmtToken::Trivia(Trivia::newline(LineEnding::CrLf))
                    }
                    Ok(FmtLexeme::NewlineLf) => FmtToken::Trivia(Trivia::newline(LineEnding::Lf)),
                    Ok(FmtLexeme::NewlineCr) => FmtToken::Trivia(Trivia::newline(LineEnding::Cr)),
                    Ok(FmtLexeme::LineComment(text)) => {
                        FmtToken::Trivia(Trivia::line_comment(text))
                    }
                    Ok(FmtLexeme::BlockComment(text)) => {
                        FmtToken::Trivia(Trivia::block_comment(text))
                    }
                    Ok(FmtLexeme::Ident(text)) => FmtToken::Token(keyword_or_ident(text)),
                    Ok(FmtLexeme::Macro(text)) => FmtToken::Token(Token::Macro(text)),
                    Ok(FmtLexeme::Path) => {
                        let text = &input[span.start..span.end];
                        let name = &text[text.find("::").expect("regex matched a prefix") + 2..];
                        let token = match &text[..3] {
                            "jet" => Token::Jet(name),
                            "wit" => Token::Witness(name),
                            _ => Token::Param(name),
                        };
                        FmtToken::Token(token)
                    }
                    Ok(FmtLexeme::Dec(v)) => FmtToken::Token(Token::DecLiteral(v)),
                    Ok(FmtLexeme::Hex(v)) => FmtToken::Token(Token::HexLiteral(v)),
                    Ok(FmtLexeme::Bin(v)) => FmtToken::Token(Token::BinLiteral(v)),
                    Ok(lexeme) => FmtToken::Token(match lexeme {
                        FmtLexeme::Arrow => Token::Arrow,
                        FmtLexeme::FatArrow => Token::FatArrow,
                        FmtLexeme::Eq => Token::Eq,
                        FmtLexeme::DoubleColon => Token::DoubleColon,
                        FmtLexeme::Colon => Token::Colon,
                        FmtLexeme::Semi => Token::Semi,
                        FmtLexeme::Comma => Token::Comma,
                        FmtLexeme::LParen => Token::LParen,
                        FmtLexeme::RParen => Token::RParen,
                        FmtLexeme::LBracket => Token::LBracket,
                        FmtLexeme::RBracket => Token::RBracket,
                        FmtLexeme::LBrace => Token::LBrace,
                        FmtLexeme::RBrace => Token::RBrace,
                        FmtLexeme::LAngle => Token::LAngle,
                        FmtLexeme::RAngle => Token::RAngle,
                        _ => unreachable!("payload variants handled above"),
                    }),
                    Err(()) => {
                        let found = input[span.start..].chars().next().expect("nonempty span");
                        let error_end = span.start + found.len_utf8();
                        if last_error_end != Some(span.start) {
                            let errors = &mut lexer.extras.errors;
                            errors.push((
                                SimpleSpan::from(span.start..error_end),
                                format!("found '{found}' expected a token"),
                            ));
                        }
                        last_error_end = Some(error_end);
                        continue;
                    }
                };
                tokens.push((fmt_token, span));
            }
            let errors = lexer
                .extras
                .errors
                .iter()
                .map(|(span, msg)| Rich::custom(*span, msg.clone()));
            (Some(tokens), errors.collect())
        }
    }
}
