//! GENERATED fast-path parser copy with bare errors: mechanically derived
//! from the `ChumskyParseBare` implementations in this module's parent
//! (`crate::parse`). Do not edit by hand. Error constructions are stubbed to
//! bare span-only errors, which is fine because this copy's errors are only a
//! signal to re-parse with the rich-error parser in the parent.
//!
//! A clean run is final (the error type cannot influence the parse result);
//! any error means: discard and re-parse rich.

use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;

use chumsky::error::Simple;
use chumsky::input::{Input, ValueInput};
use chumsky::prelude::{
    any, choice, empty, just, nested_delimiters, none_of, one_of, recursive, skip_then_retry_until,
    via_parser,
};
use chumsky::{extra, select, IterParser, Parser};
use either::Either;

use super::*;

/// The fast-path sibling of the parent's [`ChumskyParseBare`](super::ChumskyParseBare):
/// identical grammar, bare error type.
pub trait ChumskyParseBare: Sized {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseErrorBare<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>;
}

/// Bare parse errors: span and found token only. The stream borrow is
/// `'tokens`; the token data borrows `'src`.
pub type ParseErrorBare<'tokens, 'src> = extra::Err<Simple<'tokens, Token<'src>, Span>>;
type ParseError<'tokens, 'src> = ParseErrorBare<'tokens, 'src>;

/// The output of an expression is assigned to a pattern.
macro_rules! impl_parse_wrapped_string {
    ($wrapper: ident, $label: literal) => {
        impl ChumskyParseBare for $wrapper {
            fn parser<'tokens, 'src: 'tokens, I>(
            ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
            where
                I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
            {
                select! {
                    Token::Ident(ident) => Self::from_str_unchecked(ident)
                }
                .labelled($label)
            }
        }
    };
}

impl_parse_wrapped_string!(SymbolName, "unresolved symbol name");
impl_parse_wrapped_string!(FunctionName, "function name");
impl_parse_wrapped_string!(Identifier, "identifier");
impl_parse_wrapped_string!(AliasName, "alias name");
impl_parse_wrapped_string!(ModuleName, "module name");

/// Trait for generating parsers of themselves.
///
/// Replacement for previous `PestParse` trait.
/// `List<ty, bound>` would require comma inside angle brackets.
fn parse_token_with_recovery<'tokens, 'src: 'tokens, I>(
    tok: Token<'src>,
) -> impl Parser<'tokens, I, Token<'src>, ParseError<'tokens, 'src>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    just(tok.clone()).recover_with(via_parser(empty().to(tok)))
}

/// Parser with error recovery for expressions, which would always contains given delimiters.
///
/// Can track span of open delimiter (if any).
/// Can track span of open delimiter (if any).
fn delimited_with_recovery<'tokens, 'src: 'tokens, I, P, T, F>(
    parser: P,
    open: Token<'src>,
    close: Token<'src>,
    fallback: F,
) -> impl Parser<'tokens, I, T, ParseError<'tokens, 'src>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    P: Parser<'tokens, I, T, ParseError<'tokens, 'src>> + Clone,
    T: Clone + 'tokens,
    F: Fn(Span) -> T + Clone + 'tokens,
{
    just(open.clone())
        .map_with(|_, e| e.span())
        .then(parser.recover_with(via_parser(nested_delimiters(
            open.clone(),
            close.clone(),
            [
                (Token::LParen, Token::RParen),
                (Token::LBracket, Token::RBracket),
                (Token::LBrace, Token::RBrace),
                (Token::LAngle, Token::RAngle),
            ],
            fallback,
        ))))
        .then(just(close).or_not())
        // TODO: we should use information about open delimiter
        .validate(move |((open_span, content), close_token), _, emit| {
            if close_token.is_none() {
                emit.emit(Simple::new(None, open_span))
            }
            content
        })
}

impl ChumskyParseBare for AliasedType {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let atom = select! {
                Token::Ident(ident) => {
                    if ident == "bool" {
                        AliasedType::boolean()
                    } else if let Ok(uint_type) = UIntType::from_str(ident) {
                        AliasedType::from(uint_type)
                    } else if let Ok(builtin) = BuiltinAlias::from_str(ident) {
                        AliasedType::builtin(builtin)
                    } else {
                        AliasedType::alias(AliasName::from_str_unchecked(ident))
                    }
                }
        };

        let num = select! {
            Token::DecLiteral(i) => i.clone()
        }
        .labelled("decimal number")
        .recover_with(via_parser(
            none_of([Token::RAngle, Token::RBracket])
                .ignored()
                .or(empty())
                .to(Decimal::from_str_unchecked("0")),
        ));

        recursive(|ty| {
            let args = delimited_with_recovery(
                ty.clone()
                    .then_ignore(parse_token_with_recovery(Token::Comma))
                    .then(ty.clone()),
                Token::LAngle,
                Token::RAngle,
                |_| {
                    (
                        AliasedType::alias(AliasName::from_str_unchecked("error")),
                        AliasedType::alias(AliasName::from_str_unchecked("error")),
                    )
                },
            );

            let sum_type = just(Token::Ident("Either"))
                .ignore_then(args)
                .map(|(left, right)| AliasedType::either(left, right))
                .labelled("Either");

            let option_type = just(Token::Ident("Option"))
                .ignore_then(delimited_with_recovery(
                    ty.clone(),
                    Token::LAngle,
                    Token::RAngle,
                    |_| AliasedType::alias(AliasName::from_str_unchecked("error")),
                ))
                .map(AliasedType::option)
                .labelled("Option");

            let tuple = delimited_with_recovery(
                ty.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect()
                    .map(|s: Vec<AliasedType>| AliasedType::tuple(s)),
                Token::LParen,
                Token::RParen,
                |_| AliasedType::tuple(Vec::new()),
            )
            .labelled("tuple");

            let array = delimited_with_recovery(
                ty.clone()
                    .then_ignore(parse_token_with_recovery(Token::Semi))
                    .then(num.clone())
                    .map(|(ty, size)| {
                        let digits =
                            crate::str::underscore_parsing::strip_digit_separators(size.as_inner());

                        AliasedType::array(ty, usize::from_str(digits.as_ref()).unwrap_or_default())
                    }),
                Token::LBracket,
                Token::RBracket,
                |_| {
                    AliasedType::array(
                        AliasedType::alias(AliasName::from_str_unchecked("error")),
                        0,
                    )
                },
            )
            .labelled("array");

            let list = just(Token::Ident("List"))
                .ignore_then(delimited_with_recovery(
                    ty.then_ignore(parse_token_with_recovery(Token::Comma))
                        .then(num.clone().validate(|num, e, emit| -> NonZeroPow2Usize {
                            let digits = crate::str::underscore_parsing::strip_digit_separators(
                                num.as_inner(),
                            );

                            match NonZeroPow2Usize::from_str(digits.as_ref()) {
                                Ok(number) => number,
                                Err(_err) => {
                                    emit.emit(Simple::new(None, e.span()));
                                    // fallback to default value
                                    NonZeroPow2Usize::TWO
                                }
                            }
                        })),
                    Token::LAngle,
                    Token::RAngle,
                    |_| {
                        (
                            AliasedType::alias(AliasName::from_str_unchecked("error")),
                            NonZeroPow2Usize::TWO,
                        )
                    },
                ))
                .map(|(ty, size)| AliasedType::list(ty, size))
                .labelled("List");

            choice((sum_type, option_type, tuple, array, list, atom))
                .map_with(|inner, _| inner)
                .labelled("type")
        })
    }
}

impl ChumskyParseBare for Program {
    /// Parses a sequence of top-level [`Item`]s into a complete [`Program`].
    ///
    /// If an invalid item is encountered, it will safely skip the broken tokens
    /// until it finds a synchronization point. This prevents the parser from
    /// failing completely, allowing it to report multiple syntax errors across the file
    /// while substituting the unparseable sections with [`Item::Ignored`].
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let skip_until_next_item = any()
            .then(
                any()
                    .filter(|t| {
                        !matches!(
                            t,
                            Token::Pub
                                | Token::Use
                                | Token::Fn
                                | Token::Type
                                | Token::Mod
                                | Token::Enum
                        )
                    })
                    .repeated(),
            )
            .map_with(|_, _| Item::Ignored);

        <Item as ChumskyParseBare>::parser()
            .recover_with(via_parser(skip_until_next_item))
            .repeated()
            .collect::<Vec<Item>>()
            .map_with(|items, e| Program {
                items: Arc::from(items),
                span: e.span(),
            })
    }
}

impl ChumskyParseBare for Item {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        recursive(|item| {
            let func_parser = <Function as ChumskyParseBare>::parser().map(Item::Function);
            let type_parser = <TypeAlias as ChumskyParseBare>::parser().map(Item::TypeAlias);
            let use_parser = <UseDecl as ChumskyParseBare>::parser().map(Item::Use);
            let enum_parser =
                <EnumDeclaration as ChumskyParseBare>::parser().map(Item::EnumDeclaration);

            // Lazy item here
            let mod_parser = Module::bare_parser_with_items(item).map(Item::Module);

            choice((
                func_parser,
                use_parser,
                type_parser,
                enum_parser,
                mod_parser,
            ))
        })
    }
}

impl ChumskyParseBare for Function {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let visibility = just(Token::Pub)
            .to(Visibility::Public)
            .or_not()
            .map(Option::unwrap_or_default)
            .labelled("function visibility");

        let params = delimited_with_recovery(
            <FunctionParam as ChumskyParseBare>::parser()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>(),
            Token::LParen,
            Token::RParen,
            |_| Vec::new(),
        )
        .map(Arc::from)
        .labelled("function parameters");

        let ret = just(Token::Arrow)
            .ignore_then(<AliasedType as ChumskyParseBare>::parser())
            .or_not()
            .labelled("return type");

        let body = just(Token::LBrace)
            .rewind()
            .ignore_then(<Expression as ChumskyParseBare>::parser())
            .recover_with(via_parser(nested_delimiters(
                Token::LBrace,
                Token::RBrace,
                [
                    (Token::LParen, Token::RParen),
                    (Token::LBracket, Token::RBracket),
                ],
                Expression::empty,
            )))
            .labelled("function body");

        visibility
            .then_ignore(just(Token::Fn))
            .then(<FunctionName as ChumskyParseBare>::parser())
            .then(params)
            .then(ret)
            .then(body)
            .map_with(|((((visibility, name), params), ret), body), e| Self {
                visibility,
                name,
                params,
                ret,
                body,
                span: e.span(),
            })
    }
}

impl ChumskyParseBare for UseDecl {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let visibility = just(Token::Pub)
            .to(Visibility::Public)
            .or_not()
            .map(Option::unwrap_or_default);

        let first_segment = select! {
            Token::Ident(ident) => Identifier::from_str_unchecked(ident),
            Token::Crate => Identifier::from_str_unchecked(CRATE_STR),
        };

        // Parse the base path prefix (e.g., `dependency_root_path::file::`, `dependency_root_path::dir::file::`,
        // or `crate::dir::file::`). We require at least 2 segments here because a valid import needs a minimum
        // of 3 items total: the dependency root path (or `crate`), the file, and the specific item.
        //
        // With the introduction of `mod` keyword and single-file flattening, 2 total segments are now
        // valid: `crate::item`, where `crate` is the program root.
        let path = first_segment
            .then_ignore(just(Token::DoubleColon))
            .then(
                <Identifier as ChumskyParseBare>::parser()
                    .then_ignore(just(Token::DoubleColon))
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first, mut rest)| {
                let mut path = vec![first];
                path.append(&mut rest);
                path
            });

        let aliased_item = <SymbolName as ChumskyParseBare>::parser().then(
            just(Token::As)
                .ignore_then(<SymbolName as ChumskyParseBare>::parser())
                .or_not(),
        );

        let list = aliased_item
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map(UseItems::List);
        let single = aliased_item.map(UseItems::Single);
        let items = choice((list, single));

        visibility
            .then_ignore(just(Token::Use))
            .then(path)
            .then(items)
            .then_ignore(just(Token::Semi))
            .map_with(|((visibility, path), items), e| Self {
                visibility,
                path,
                items,
                span: e.span(),
            })
    }
}

impl ChumskyParseBare for FunctionParam {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let identifier = <Identifier as ChumskyParseBare>::parser();

        let ty = <AliasedType as ChumskyParseBare>::parser();

        identifier
            .then_ignore(just(Token::Colon))
            .then(ty)
            .map_with(|(identifier, ty), e| Self {
                identifier,
                ty,
                span: e.span(),
            })
    }
}

impl Statement {
    fn bare_parser<'tokens, 'src: 'tokens, I, E>(
        expr: E,
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
        E: Parser<'tokens, I, Expression, ParseError<'tokens, 'src>> + Clone + 'tokens,
    {
        let assignment = Assignment::bare_parser(expr.clone()).map(Statement::Assignment);

        let expression = expr.map(Statement::Expression);

        choice((assignment, expression))
    }
}

impl Assignment {
    fn bare_parser<'tokens, 'src: 'tokens, I, E>(
        expr: E,
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
        E: Parser<'tokens, I, Expression, ParseError<'tokens, 'src>> + Clone + 'tokens,
    {
        just(Token::Let)
            .ignore_then(<Pattern as ChumskyParseBare>::parser())
            .then_ignore(parse_token_with_recovery(Token::Colon))
            .then(<AliasedType as ChumskyParseBare>::parser())
            .then_ignore(parse_token_with_recovery(Token::Eq))
            .then(expr)
            .map_with(|((pattern, ty), expression), e| Self {
                pattern,
                ty,
                expression,
                span: e.span(),
            })
    }
}

impl ChumskyParseBare for Pattern {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        recursive(|pat| {
            let variable = <Identifier as ChumskyParseBare>::parser().map(Pattern::Identifier);

            let ignore = select! {
                Token::Ident("_") => Pattern::Ignore,
            };

            let tuple = delimited_with_recovery(
                pat.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>(),
                Token::LParen,
                Token::RParen,
                |_| Vec::new(),
            )
            .map(Pattern::tuple);

            let array = delimited_with_recovery(
                pat.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>(),
                Token::LBracket,
                Token::RBracket,
                |_| Vec::new(),
            )
            .map(Pattern::array);

            choice((ignore, variable, tuple, array)).labelled("pattern")
        })
    }
}

impl Call {
    fn bare_parser<'tokens, 'src: 'tokens, I, E>(
        expr: E,
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
        E: Parser<'tokens, I, Expression, ParseError<'tokens, 'src>> + Clone + 'tokens,
    {
        let args = delimited_with_recovery(
            expr.separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>(),
            Token::LParen,
            Token::RParen,
            |_| Vec::new(),
        )
        .map(Arc::from)
        .labelled("call arguments");

        <CallName as ChumskyParseBare>::parser()
            .then(args)
            .map_with(|(name, args), e| Self {
                name,
                args,
                span: e.span(),
            })
    }
}

impl ChumskyParseBare for CallName {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let turbofish_start = just(Token::DoubleColon).then(just(Token::LAngle)).ignored();

        let generics_close = just(Token::RAngle);

        let type_cast = just(Token::LAngle)
            .ignore_then(<AliasedType as ChumskyParseBare>::parser())
            .then_ignore(generics_close.clone())
            .then_ignore(just(Token::DoubleColon))
            .then_ignore(just(Token::Ident("into")))
            .map(CallName::TypeCast);

        let builtin_generic_ty = |name: &'static str, ctor: fn(AliasedType) -> Self| {
            just(Token::Ident(name))
                .ignore_then(turbofish_start.clone())
                .ignore_then(<AliasedType as ChumskyParseBare>::parser())
                .then_ignore(generics_close.clone())
                .map(ctor)
        };

        let unwrap_left = builtin_generic_ty("unwrap_left", CallName::UnwrapLeft);
        let unwrap_right = builtin_generic_ty("unwrap_right", CallName::UnwrapRight);
        let is_none = builtin_generic_ty("is_none", CallName::IsNone);

        let fold = just(Token::Ident("fold"))
            .ignore_then(turbofish_start.clone())
            .ignore_then(<FunctionName as ChumskyParseBare>::parser())
            .then_ignore(parse_token_with_recovery(Token::Comma))
            .then(select! { Token::DecLiteral(s) => s }.labelled("list size"))
            .then_ignore(generics_close.clone())
            .validate(|(func, bound_str), e, emit| {
                let digits =
                    crate::str::underscore_parsing::strip_digit_separators(bound_str.as_inner());

                let bound = match digits.parse::<usize>() {
                    Ok(num) => match NonZeroPow2Usize::new(num) {
                        Some(val) => val,
                        None => {
                            emit.emit(Simple::new(None, e.span()));
                            NonZeroPow2Usize::TWO
                        }
                    },
                    Err(_) => {
                        emit.emit(Simple::new(None, e.span()));
                        NonZeroPow2Usize::TWO
                    }
                };

                CallName::Fold(func, bound)
            });

        let array_fold = just(Token::Ident("array_fold"))
            .ignore_then(turbofish_start.clone())
            .ignore_then(<FunctionName as ChumskyParseBare>::parser())
            .then_ignore(parse_token_with_recovery(Token::Comma))
            .then(select! { Token::DecLiteral(s) => s }.labelled("array size"))
            .then_ignore(generics_close.clone())
            .validate(|(func, size_str), e, emit| {
                let digits =
                    crate::str::underscore_parsing::strip_digit_separators(size_str.as_inner());

                let size = match digits.parse::<usize>() {
                    Ok(0) => {
                        emit.emit(Simple::new(None, e.span()));
                        NonZeroUsize::new(1).unwrap()
                    }
                    Ok(n) => NonZeroUsize::new(n).unwrap(),
                    Err(_) => {
                        emit.emit(Simple::new(None, e.span()));
                        NonZeroUsize::new(1).unwrap()
                    }
                };

                CallName::ArrayFold(func, size)
            });

        let for_while = just(Token::Ident("for_while"))
            .ignore_then(turbofish_start.clone())
            .ignore_then(<FunctionName as ChumskyParseBare>::parser())
            .then_ignore(generics_close.clone())
            .map(CallName::ForWhile);

        let simple_builtins = select! {
            Token::Ident("unwrap") => CallName::Unwrap,
            Token::Macro("assert!") => CallName::Assert,
            Token::Macro("panic!") => CallName::Panic,
            Token::Macro("dbg!") => CallName::Debug,
        };

        let jet = select! { Token::Jet(s) => JetName::from_str_unchecked(s) }.map(CallName::Jet);

        let custom_func = <FunctionName as ChumskyParseBare>::parser().map(CallName::Custom);

        choice((
            type_cast,
            unwrap_left,
            unwrap_right,
            is_none,
            fold,
            array_fold,
            for_while,
            simple_builtins,
            jet,
            custom_func,
        ))
    }
}

impl ChumskyParseBare for TypeAlias {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let visibility = just(Token::Pub)
            .to(Visibility::Public)
            .or_not()
            .map(Option::unwrap_or_default);

        let name = <AliasName as ChumskyParseBare>::parser()
            .validate(|name, e, emit| {
                let ident = name.as_str();
                let known_type = if ident == "bool" {
                    Some(AliasedType::boolean())
                } else if let Ok(uint_type) = UIntType::from_str(ident) {
                    Some(AliasedType::from(uint_type))
                } else if let Ok(builtin) = BuiltinAlias::from_str(ident) {
                    Some(AliasedType::builtin(builtin))
                } else {
                    None
                };

                if known_type.is_some() {
                    emit.emit(Simple::new(None, e.span()));
                }
                name
            })
            .map_with(|name, e| (name, e.span()));

        visibility
            .then(
                just(Token::Type)
                    .ignore_then(name)
                    .then_ignore(parse_token_with_recovery(Token::Eq))
                    .then(<AliasedType as ChumskyParseBare>::parser())
                    .then_ignore(just(Token::Semi)),
            )
            .map_with(|(visibility, (name, ty)), e| Self {
                visibility,
                name: name.0,
                ty,
                span: e.span(),
            })
    }
}

/// Identifiers of the built-in binary match patterns.
/// An enum may not use them as its name, because e.g. `Left::A` in a match arm would parse as the built-in `Left(..)` pattern.
pub(crate) const RESERVED_PATTERN_NAMES: [&str; 4] = ["Left", "Right", "Some", "None"];

impl ChumskyParseBare for EnumDeclaration {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let visibility = just(Token::Pub)
            .to(Visibility::Public)
            .or_not()
            .map(Option::unwrap_or_default);

        let name = <AliasName as ChumskyParseBare>::parser().try_map(|name, span| {
            if RESERVED_PATTERN_NAMES.contains(&name.as_str()) {
                return Err(Simple::new(None, span));
            }
            // Reserved type names are rejected via the shared list, which
            // also covers the generic constructors (`Either`, `Option`,
            // `List`): `enum Signature` or `enum Option` would make
            // constructions name the enum while type annotations resolve
            // to the builtin, and the ABI would report the bare name
            // ambiguously.
            if crate::str::is_reserved_alias_name(name.as_str()) {
                return Err(Simple::new(None, span));
            }
            Ok(name)
        });

        let payload = <AliasedType as ChumskyParseBare>::parser()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .at_least(1)
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .or_not()
            .map(|payload| Arc::from(payload.unwrap_or_default()));

        let variant = <Identifier as ChumskyParseBare>::parser()
            .then(payload)
            .map_with(|(name, payload), e| EnumVariant {
                name,
                payload,
                span: e.span(),
            });

        let variants = variant
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map(Arc::from);

        visibility
            .then_ignore(just(Token::Enum))
            .then(name)
            .then(variants)
            .map_with(|((visibility, name), variants), e| Self {
                visibility,
                name,
                variants,
                span: e.span(),
            })
    }
}

impl ChumskyParseBare for Expression {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        recursive(|expr| {
            let block = {
                let statement = Statement::bare_parser(expr.clone()).then_ignore(just(Token::Semi));

                let block_recovery = nested_delimiters(
                    Token::LBrace,
                    Token::RBrace,
                    [
                        (Token::LParen, Token::RParen),
                        (Token::LAngle, Token::RAngle),
                        (Token::LBracket, Token::RBracket),
                    ],
                    |span| Expression::empty(span).inner().clone(),
                );

                let statements = statement
                    .repeated()
                    .collect::<Vec<_>>()
                    .map(Arc::from)
                    .recover_with(skip_then_retry_until(
                        block_recovery.ignored().or(any().ignored()),
                        one_of([Token::Semi, Token::RParen, Token::RBracket, Token::RBrace])
                            .ignored(),
                    ));

                let final_expr = expr.clone().map(Arc::new).or_not();

                delimited_with_recovery(
                    statements.then(final_expr),
                    Token::LBrace,
                    Token::RBrace,
                    |_| (Arc::from(Vec::new()), None),
                )
                .map(|(stmts, end_expr)| ExpressionInner::Block(stmts, end_expr))
            };

            let single = SingleExpression::bare_parser(expr.clone()).map(ExpressionInner::Single);

            choice((block, single))
                .map_with(|inner, e| Expression {
                    inner,
                    span: e.span(),
                })
                .labelled("expression")
        })
    }
}

impl SingleExpression {
    fn bare_parser<'tokens, 'src: 'tokens, I, E>(
        expr: E,
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
        E: Parser<'tokens, I, Expression, ParseError<'tokens, 'src>> + Clone + 'tokens,
    {
        let wrapper = |name: &'static str| {
            select! { Token::Ident(i) if i == name => i }.ignore_then(
                expr.clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
        };

        let left =
            wrapper("Left").map(|e| SingleExpressionInner::Either(Either::Left(Arc::new(e))));

        let right =
            wrapper("Right").map(|e| SingleExpressionInner::Either(Either::Right(Arc::new(e))));

        let some = wrapper("Some").map(|e| SingleExpressionInner::Option(Some(Arc::new(e))));

        // Bare `None` is the option literal only when no `::` follows:
        // `None::A` is a construction of an enum aliased as `None`, and
        // must fall through to the enum-construction alternative.
        let none = select! { Token::Ident("None") => SingleExpressionInner::Option(None) }
            .then_ignore(just(Token::DoubleColon).not().rewind());

        let boolean = select! { Token::Bool(b) => SingleExpressionInner::Boolean(b) };

        let comma_separated = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>();

        let array = delimited_with_recovery(
            comma_separated.clone(),
            Token::LBracket,
            Token::RBracket,
            |_| Vec::new(),
        )
        .map(|es| SingleExpressionInner::Array(Arc::from(es)));

        let list = just(Token::Macro("list!"))
            .ignore_then(delimited_with_recovery(
                comma_separated.clone(),
                Token::LBracket,
                Token::RBracket,
                |_| Vec::new(),
            ))
            .map(|es| SingleExpressionInner::List(Arc::from(es)));

        let tuple = delimited_with_recovery(
            comma_separated.clone(),
            Token::LParen,
            Token::RParen,
            |_| Vec::new(),
        )
        .map(|es| SingleExpressionInner::Tuple(Arc::from(es)));

        let literal = select! {
            Token::DecLiteral(s) => SingleExpressionInner::Decimal(s),
            Token::HexLiteral(s) => SingleExpressionInner::Hexadecimal(s),
            Token::BinLiteral(s) => SingleExpressionInner::Binary(s),
            Token::Witness(s) => SingleExpressionInner::Witness(TemplateProgramWitness::witness_from_str(s)),
            Token::Param(s) => SingleExpressionInner::Parameter(TemplateProgramWitness::parameter_from_str(s)),
        };

        // Enum variant construction: `Path::To::Enum::Variant(args..)`.
        // At least one `::` distinguishes the path from variables and calls.
        // The built-in wrappers (Left, Some, ...) require `(` directly after
        // their name, so they never reach this alternative.
        let enum_construction = <Identifier as ChumskyParseBare>::parser()
            .separated_by(just(Token::DoubleColon))
            .at_least(2)
            .collect::<Vec<Identifier>>()
            .then(
                comma_separated
                    .clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .or_not(),
            )
            .map_with(|(mut segments, args), e| {
                let variant = segments.pop().expect("the parser requires two segments");
                EnumConstruction {
                    enum_path: Arc::from(segments),
                    variant,
                    args: Arc::from(args.unwrap_or_default()),
                    span: e.span(),
                }
            })
            .map(SingleExpressionInner::EnumConstruction);

        let call = Call::bare_parser(expr.clone()).map(SingleExpressionInner::Call);

        let match_expr = match_expr_parser(expr.clone());

        let variable =
            <Identifier as ChumskyParseBare>::parser().map(SingleExpressionInner::Variable);

        // Expression delimeted by parentheses
        let expression = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map(|es| SingleExpressionInner::Expression(Arc::from(es)));

        choice((
            left,
            right,
            some,
            none,
            boolean,
            match_expr,
            expression,
            list,
            array,
            tuple,
            enum_construction,
            call,
            literal,
            variable,
        ))
        .map_with(|inner, e| Self {
            inner,
            span: e.span(),
        })
    }
}

impl ChumskyParseBare for MatchPattern {
    fn parser<'tokens, 'src: 'tokens, I>(
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let wrapper = |name: &'static str, ctor: fn(Pattern, AliasedType) -> Self| {
            select! { Token::Ident(i) if i == name => i }
                .ignore_then(delimited_with_recovery(
                    <Pattern as ChumskyParseBare>::parser()
                        .then_ignore(just(Token::Colon))
                        .then(<AliasedType as ChumskyParseBare>::parser()),
                    Token::LParen,
                    Token::RParen,
                    |_| {
                        (
                            Pattern::Ignore,
                            AliasedType::alias(AliasName::from_str_unchecked("error")),
                        )
                    },
                ))
                .map(move |(id, ty)| ctor(id, ty))
        };

        choice((
            wrapper("Left", MatchPattern::Left),
            wrapper("Right", MatchPattern::Right),
            wrapper("Some", MatchPattern::Some),
            select! { Token::Ident("None") => MatchPattern::None },
            select! { Token::Bool(true) => MatchPattern::True },
            select! { Token::Bool(false) => MatchPattern::False },
        ))
    }
}

/// Head of an enum match arm, before the arm body is attached: `EnumName::Variant` with optional payload bindings.
/// plus the arm body and its source boundaries.
/// Parser for the head of an enum match arm: `EnumName::Variant` with
/// optional payload bindings `(pattern: Type, ...)`.
///
/// A non-reserved head identifier commits without backtracking: the
/// `select!` guard fails without consuming the token. A reserved pattern
/// name (`Left`, `Some`, ...) heads an enum path only when `::` follows,
/// so an alias of an enum that shadows a pattern name stays matchable
/// while `Left(x)` remains the built-in pattern.
fn enum_arm_head_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, EnumArmHead, ParseError<'tokens, 'src>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    let bindings = <Pattern as ChumskyParseBare>::parser()
        .then_ignore(just(Token::Colon))
        .then(<AliasedType as ChumskyParseBare>::parser())
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .at_least(1)
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LParen), just(Token::RParen))
        .or_not()
        .map(|bindings| Arc::from(bindings.unwrap_or_default()));

    let head_ident = choice((
        select! { Token::Ident(name) if !RESERVED_PATTERN_NAMES.contains(&name) => Identifier::from_str_unchecked(name) },
        // The rewind leaves the `::` for the shared `then_ignore` below.
        select! { Token::Ident(name) => Identifier::from_str_unchecked(name) }
            .then_ignore(just(Token::DoubleColon).rewind()),
    ));

    head_ident
        .then_ignore(just(Token::DoubleColon))
        .then(
            <Identifier as ChumskyParseBare>::parser()
                .separated_by(just(Token::DoubleColon))
                .at_least(1)
                .collect::<Vec<Identifier>>(),
        )
        .then(bindings)
        .map(|((first, mut rest), bindings)| {
            let variant = rest.pop().expect("the parser requires one segment");
            let mut enum_path = vec![first];
            enum_path.extend(rest);
            EnumArmHead {
                enum_path: Arc::from(enum_path),
                variant,
                bindings,
            }
        })
}

/// Parser for one match arm. An arm head, `=>`, the arm body, and the
/// comma that is required unless the body is a block expression.
/// comma that is required unless the body is a block expression.
fn match_arm_parser<'tokens, 'src: 'tokens, I, E>(
    expr: E,
) -> impl Parser<'tokens, I, ParsedMatchArm, ParseError<'tokens, 'src>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    E: Parser<'tokens, I, Expression, ParseError<'tokens, 'src>> + Clone + 'tokens,
{
    let arm_head = choice((
        enum_arm_head_parser().map(Either::Left),
        <MatchPattern as ChumskyParseBare>::parser().map(Either::Right),
    ));

    arm_head
        .then_ignore(just(Token::FatArrow))
        .then(expr.map(Arc::new))
        .then(just(Token::Comma).or_not())
        .validate(|((head, expression), comma), e, emitter| {
            let is_block = matches!(expression.as_ref().inner(), ExpressionInner::Block(_, _));
            if !is_block && comma.is_none() {
                emitter.emit(Simple::new(None, e.span()));
            }
            (head, expression, e.span())
        })
}

/// A binary match with dummy arms, standing in for a malformed match so
/// parsing can continue after its error was reported.
/// parsing can continue after its error was reported.
fn placeholder_match(scrutinee: Arc<Expression>, span: Span) -> SingleExpressionInner {
    let fallback_arm = MatchArm {
        expression: Arc::new(Expression::empty(Span::DUMMY)),
        pattern: MatchPattern::False,
        span: Span::DUMMY,
    };
    SingleExpressionInner::Match(Match {
        scrutinee,
        left: fallback_arm.clone(),
        right: fallback_arm,
        span,
    })
}

/// Do the two patterns complement each other in canonical order
/// (`Left`/`Right`, `None`/`Some`, `false`/`true`)?
/// (`Left`/`Right`, `None`/`Some`, `false`/`true`)?
fn patterns_in_canonical_order(left: &MatchPattern, right: &MatchPattern) -> bool {
    matches!(
        (left, right),
        (MatchPattern::Left(..), MatchPattern::Right(..))
            | (MatchPattern::None, MatchPattern::Some(..))
            | (MatchPattern::False, MatchPattern::True)
    )
}

/// Assemble parsed arms into an [`EnumMatch`] or a binary [`Match`] node.
///
/// Malformed arms return the error to report together with a fallback
/// node, so the caller emits exactly one error per malformed match.
/// Running bad arms through the pattern pairing would emit a second,
/// bogus "incompatible match arms" error for the same span.
/// bogus "incompatible match arms" error for the same span.
fn assemble_match_arms(
    scrutinee: Arc<Expression>,
    arms: Vec<ParsedMatchArm>,
    span: Span,
) -> Result<SingleExpressionInner, Box<(Diagnostic, SingleExpressionInner)>> {
    if arms.is_empty() {
        return Err(Box::new((
            Error::Grammar {
                msg: "match expression has no arms".to_string(),
            }
            .with_span(span),
            placeholder_match(scrutinee, span),
        )));
    }

    // Split arms by the classification the arm heads carry. Any number of
    // enum arms (including one) routes to the enum match, whose analysis
    // reports missing variants. The binary arm-count error below would be misleading there.
    let (enum_arms, builtin_arms): (Vec<EnumMatchArm>, Vec<MatchArm>) = arms
        .into_iter()
        .partition_map(|(head, expression, arm_span)| match head {
            Either::Left(head) => Either::Left(head.into_arm(expression, arm_span)),
            Either::Right(pattern) => Either::Right(MatchArm {
                pattern,
                expression,
                span: arm_span,
            }),
        });

    if builtin_arms.is_empty() {
        return Ok(SingleExpressionInner::EnumMatch(EnumMatch {
            scrutinee,
            arms: Arc::from(enum_arms),
            span,
        }));
    }

    if let Some(enum_arm) = enum_arms.first() {
        // Mixed arms: name the clash instead of an arm-count error.
        return Err(Box::new((
            Error::Grammar {
                msg: format!(
                    "match arms mix the enum variant pattern `{}` \
                     with the built-in pattern `{}`",
                    enum_arm,
                    builtin_arms[0].pattern()
                ),
            }
            .with_span(span),
            placeholder_match(scrutinee, span),
        )));
    }

    // Binary match: exactly 2 built-in arms, in either order.
    let Ok([first, second]) = <[MatchArm; 2]>::try_from(builtin_arms) else {
        return Err(Box::new((
            Error::Grammar {
                msg: "binary match requires exactly 2 arms".to_string(),
            }
            .with_span(span),
            placeholder_match(scrutinee, span),
        )));
    };

    let (left, right) = if patterns_in_canonical_order(first.pattern(), second.pattern()) {
        (first, second)
    } else if patterns_in_canonical_order(second.pattern(), first.pattern()) {
        (second, first)
    } else {
        let error = Error::IncompatibleMatchArms {
            first: Box::new(first.pattern().clone()),
            second: Box::new(second.pattern().clone()),
        }
        .with_span(span);
        // The arms still form a match; keep them so analysis can continue.
        let node = SingleExpressionInner::Match(Match {
            scrutinee,
            left: first,
            right: second,
            span,
        });
        return Err(Box::new((error, node)));
    };

    Ok(SingleExpressionInner::Match(Match {
        scrutinee,
        left,
        right,
        span,
    }))
}

/// Parser for `match` expressions.
///
/// Handles both binary matches (exactly 2 arms: Left/Right, None/Some,
/// false/true) and enum matches (arms of the form `EnumName::Variant`).
/// Dispatches to [`Match`] or [`EnumMatch`] based on the patterns found.
/// Dispatches to [`Match`] or [`EnumMatch`] based on the patterns found.
fn match_expr_parser<'tokens, 'src: 'tokens, I, E>(
    expr: E,
) -> impl Parser<'tokens, I, SingleExpressionInner, ParseError<'tokens, 'src>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    E: Parser<'tokens, I, Expression, ParseError<'tokens, 'src>> + Clone + 'tokens,
{
    let arms = delimited_with_recovery(
        match_arm_parser(expr.clone())
            .repeated()
            .collect::<Vec<_>>(),
        Token::LBrace,
        Token::RBrace,
        |_| vec![],
    );

    just(Token::Match)
        .ignore_then(expr.map(Arc::new))
        .then(arms)
        .validate(|(scrutinee, arms), e, emit| {
            match assemble_match_arms(scrutinee, arms, e.span()) {
                Ok(node) => node,
                Err(boxed) => {
                    let (_error, fallback) = *boxed;
                    emit.emit(Simple::new(None, e.span()));
                    fallback
                }
            }
        })
}

impl Module {
    pub fn bare_parser_with_items<'tokens, 'src: 'tokens, I>(
        item_parser: impl Parser<'tokens, I, Item, ParseError<'tokens, 'src>> + Clone + 'tokens,
    ) -> impl Parser<'tokens, I, Self, ParseError<'tokens, 'src>> + Clone
    where
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    {
        let visibility = just(Token::Pub)
            .to(Visibility::Public)
            .or_not()
            .map(Option::unwrap_or_default);

        let name = <ModuleName as ChumskyParseBare>::parser().map_with(|name, e| (name, e.span()));

        let items = item_parser
            .repeated()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .recover_with(via_parser(nested_delimiters(
                Token::LBrace,
                Token::RBrace,
                [
                    (Token::LParen, Token::RParen),
                    (Token::LBracket, Token::RBracket),
                ],
                |_| Vec::new(),
            )))
            .map(Arc::from);

        visibility
            .then(just(Token::Mod).ignore_then(name).then(items))
            .map_with(|(visibility, (name, items)), e| Self {
                visibility,
                name: name.0,
                items,
                span: e.span(),
            })
            .validate(|module, _, emit| {
                // TODO: Enums may only be declared at the top level of a file (done so to reduce scope of the PR).
                // The bare name is the enum's identity in the ABI, and a module path would obscure it.
                // Direct children suffice. Nested modules validate their own items.
                for item in module.items() {
                    if let Item::EnumDeclaration(decl) = item {
                        emit.emit(Simple::new(None, *decl.as_ref()));
                    }
                }
                module
            })
    }
}

/// Fast-path parse of a whole program with bare errors. Returns `None` on any
/// parse error; the caller re-parses with the rich-error parser in `crate::parse`.
/// The error type cannot influence the parse result, so a clean run is final.
pub(super) fn parse_program(
    file_id: usize,
    src: &str,
    tokens: &[(crate::lexer::Token<'_>, Span)],
) -> Option<Program> {
    let eoi = Span::eof(file_id, src.len());
    let (ast, parse_errs) = <Program as ChumskyParseBare>::parser()
        .parse(tokens[..].map(eoi, |(t, s)| (t, s)))
        .into_output_errors();
    if parse_errs.is_empty() {
        ast
    } else {
        None
    }
}
