use crate::config::ConfigItem;
use anyhow::{anyhow, Result};

type ParseResult<'a, Output> = Result<(&'a str, Output), &'a str>;

trait Parser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output>;
}

impl<'a, F, Output> Parser<'a, Output> for F
where
    F: Fn(&'a str) -> ParseResult<Output>,
{
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self(input)
    }
}

fn match_literal<'a>(expected: &'static str) -> impl Parser<'a, ()> {
    move |input: &'a str| match input.starts_with(expected) {
        true => Ok((&input[expected.len()..], ())),
        false => Err(input),
    }
}

fn match_space(input: &str) -> ParseResult<()> {
    match_literal(" ").parse(input)
}

fn ignore_whitespace(mut input: &str) -> &str {
    while let Ok((i, _)) = match_space(input) {
        input = i;
    }
    input
}

fn match_until_literal<'a>(expected: &'static str) -> impl Parser<'a, &'a str> {
    move |input: &'a str| match input.find(expected) {
        Some(index) => Ok((&input[index..], &input[..index])),
        None => Err(input),
    }
}

fn identifier(input: &str) -> ParseResult<&str> {
    let mut char_indices = input.char_indices();

    let mut index = match char_indices.next() {
        Some((i, c)) if c.is_alphabetic() || c == '_' || c == '@' => i,
        _ => return Err(input),
    };

    for (i, c) in input.char_indices() {
        if c.is_alphanumeric() || c == '_' || c == '@' {
            index = i;
        } else {
            break;
        }
    }

    Ok((&input[index + 1..], &input[..=index]))
}

fn string_literal(input: &str) -> ParseResult<&str> {
    let match_quote = match_literal("\"");
    let match_until_quote = match_until_literal("\"");

    match_quote
        .parse(input)
        .and_then(|(i, _)| match_until_quote.parse(i))
        .and_then(|(i, contents)| match_quote.parse(i).map(|(i, _)| (i, contents)))
}

fn is_empty_or_comment(input: &str) -> bool {
    input.is_empty() || match_literal("//").parse(input).is_ok()
}

struct State<S> {
    state: S,
}

enum MaybeTransition<S> {
    Next(State<S>),
    Done(Option<ConfigItem>),
    Err(anyhow::Error),
}

impl<S> MaybeTransition<S> {
    fn map<NS>(self, f: impl Fn(State<S>) -> MaybeTransition<NS>) -> MaybeTransition<NS> {
        match self {
            MaybeTransition::Next(s) => f(s),
            MaybeTransition::Done(ci) => MaybeTransition::Done(ci),
            MaybeTransition::Err(e) => MaybeTransition::Err(e),
        }
    }

    fn map_result(
        self,
        f: impl Fn(State<S>) -> Result<Option<ConfigItem>>,
    ) -> Result<Option<ConfigItem>> {
        match self {
            MaybeTransition::Next(s) => f(s),
            MaybeTransition::Done(ci) => Ok(ci),
            MaybeTransition::Err(e) => Err(e),
        }
    }
}

struct IdentParser<'a> {
    input: &'a str,
}

impl<'a> State<IdentParser<'a>> {
    fn init(input: &'a str) -> MaybeTransition<IdentParser<'a>> {
        let input = ignore_whitespace(input);

        if is_empty_or_comment(input) {
            MaybeTransition::Done(None)
        } else {
            MaybeTransition::Next(State {
                state: IdentParser { input },
            })
        }
    }

    fn next(self) -> MaybeTransition<CvarParser<'a>> {
        let (input, ident) = match identifier(self.state.input) {
            Ok(r) => r,
            Err(i) => return MaybeTransition::Err(anyhow!("invalid identifier, {}", i)),
        };

        let input = ignore_whitespace(input);
        if is_empty_or_comment(input) {
            MaybeTransition::Done(Some(ConfigItem::Command(ident.to_owned())))
        } else {
            MaybeTransition::Next(State {
                state: CvarParser { input, cvar: ident },
            })
        }
    }
}

struct CvarParser<'a> {
    input: &'a str,
    cvar: &'a str,
}

impl<'a> State<CvarParser<'a>> {
    fn next(self) -> MaybeTransition<BindParser<'a>> {
        let (input, val) = match string_literal(self.state.input) {
            Ok(r) => r,
            Err(i) => return MaybeTransition::Err(anyhow!("invalid string literal, {}", i)),
        };

        let input = ignore_whitespace(input);
        if is_empty_or_comment(input) {
            MaybeTransition::Done(Some(ConfigItem::Cvar(
                self.state.cvar.to_owned(),
                val.to_owned(),
            )))
        } else {
            MaybeTransition::Next(State {
                state: BindParser {
                    input,
                    bind_command: self.state.cvar,
                    key: val,
                },
            })
        }
    }
}

struct BindParser<'a> {
    input: &'a str,
    bind_command: &'a str,
    key: &'a str,
}

impl<'a> State<BindParser<'a>> {
    fn next(self) -> Result<Option<ConfigItem>> {
        if self.state.bind_command != "bind" {
            return Err(anyhow!(
                "expected command 'bind', found {} in {}",
                self.state.bind_command,
                self.state.input
            ));
        }

        let (input, val) = string_literal(self.state.input)
            .map_err(|i| anyhow!("invalid string literal, {}", i))?;

        let input = ignore_whitespace(input);
        if is_empty_or_comment(input) {
            Ok(Some(ConfigItem::Bind(
                self.state.key.to_owned(),
                val.to_owned(),
            )))
        } else {
            Err(anyhow!("invalid end of line, {}", input))
        }
    }
}

pub fn parse_line(line: &str) -> Result<Option<ConfigItem>> {
    State::<IdentParser>::init(line)
        .map(|s| s.next())
        .map(|s| s.next())
        .map_result(|s| s.next())
}
