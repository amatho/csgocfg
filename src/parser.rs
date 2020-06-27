use crate::config::ConfigItem;
use anyhow::{anyhow, bail};

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
    let mut index = 0;

    for (i, c) in input.char_indices() {
        if c.is_alphanumeric() || c == '_' || c == '@' {
            index = i;
        } else {
            break;
        }
    }

    if index == 0 {
        Err(input)
    } else {
        Ok((&input[index + 1..], &input[..=index]))
    }
}

fn string_literal(input: &str) -> ParseResult<&str> {
    let match_quote = match_literal("\"");
    let match_until_quote = match_until_literal("\"");

    match_quote
        .parse(input)
        .and_then(|(i, _)| match_until_quote.parse(i))
        .and_then(|(i, contents)| match_quote.parse(i).map(|(i, _)| (i, contents)))
}

fn parse_bind(input: &str) -> ParseResult<(&str, &str)> {
    let (input, key) = string_literal(input)?;

    let input = ignore_whitespace(input);
    let (input, bind) = string_literal(input)?;

    Ok((input, (key, bind)))
}

pub fn parse_line(line: &str) -> Result<Option<ConfigItem>, anyhow::Error> {
    let match_comment = match_literal("//");

    let input = ignore_whitespace(line);
    let (input, ident) = match identifier(input) {
        Ok(r) => r,
        Err(i) => bail!("failed to parse, {}", i),
    };
    let mut input = ignore_whitespace(input);

    let item = if input.is_empty() {
        Some(ConfigItem::Command(ident.to_owned()))
    } else if ident == "bind" {
        let (i, (key, bind)) = parse_bind(input).map_err(|i| anyhow!("invalid bind, {}", i))?;
        input = i;
        Some(ConfigItem::Bind(key.to_owned(), bind.to_owned()))
    } else if let Ok((i, contents)) = string_literal(input) {
        input = i;
        Some(ConfigItem::Cvar(ident.to_owned(), contents.to_owned()))
    } else {
        None
    };

    let input = ignore_whitespace(input);

    if input.is_empty() || match_comment.parse(input).is_ok() {
        Ok(item)
    } else {
        Err(anyhow!("invalid end of line, {}", input))
    }
}
