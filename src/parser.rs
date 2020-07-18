use crate::config::ConfigItem;
use thiserror::Error;

type ParseResult<'a, Output> = Result<(&'a str, Output), &'a str>;

fn match_literal(expected: &'static str) -> impl Fn(&str) -> ParseResult<&'static str> {
    move |input: &str| {
        let n = expected.len();
        match input
            .chars()
            .zip(expected.chars())
            .position(|(a, b)| a != b)
        {
            None if input.len() >= n => Ok((&input[n..], expected)),
            _ => Err(input),
        }
    }
}

fn ignore_whitespace(input: &str) -> &str {
    match input.chars().position(|c| c != ' ') {
        Some(index) => &input[index..],
        _ => &input[input.len()..],
    }
}

fn match_until_char(expected: char) -> impl Fn(&str) -> ParseResult<&str> {
    move |input: &str| match input.chars().position(|c| c == expected) {
        Some(index) => Ok((&input[index..], &input[..index])),
        _ => Err(input),
    }
}

fn identifier(input: &str) -> ParseResult<&str> {
    let mut chars = input.chars();

    if chars
        .next()
        .filter(|&c| c.is_alphabetic() || c == '_' || c == '@')
        .is_none()
    {
        return Err(input);
    };

    match chars.position(|c| !(c.is_alphanumeric() || c == '_' || c == '@')) {
        Some(index) => Ok((&input[index + 1..], &input[..index + 1])),
        None => Ok((&input[input.len()..], input)),
    }
}

fn string_literal(input: &str) -> ParseResult<&str> {
    let match_quote = match_literal("\"");
    let match_until_quote = match_until_char('"');

    match_quote(input)
        .and_then(|(i, _)| match_until_quote(i))
        .and_then(|(i, contents)| match_quote(i).map(|(i, _)| (i, contents)))
}

fn is_empty_or_comment(input: &str) -> bool {
    input.is_empty() || match_literal("//")(input).is_ok()
}

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("invalid identifier `{0}`")]
    InvalidIdentifier(String),
    #[error("invalid string literal (expected `\"...\"`, found `{0}`)")]
    InvalidStringLiteral(String),
    #[error("unexpected end of line, `{0}`")]
    UnexpectedEndOfLine(String),
}

pub fn parse_line(line: &str) -> Result<Option<ConfigItem>, ParseError> {
    // A line looks like
    // command {argument 1} {argument 2} [COMMENT]
    // where the number of arguments can be either 0, 1, or 2.
    // Whitespace is optional and can appear zero or more times between the tokens above.

    // [COMMENT]
    let input = ignore_whitespace(line);
    if is_empty_or_comment(input) {
        return Ok(None);
    }

    // command [COMMENT]
    let (input, cmd) =
        identifier(input).map_err(|i| ParseError::InvalidIdentifier(i.to_owned()))?;
    let input = ignore_whitespace(input);
    if is_empty_or_comment(input) {
        return Ok(Some(ConfigItem::Command(cmd.to_owned())));
    }

    // command "argument 1" [COMMENT]
    let (input, arg1) =
        string_literal(input).map_err(|i| ParseError::InvalidStringLiteral(i.to_owned()))?;
    let input = ignore_whitespace(input);
    if is_empty_or_comment(input) {
        return Ok(Some(ConfigItem::Cvar(cmd.to_owned(), arg1.to_owned())));
    }

    // command "argument 1" "argument 2" [COMMENT]
    let (input, arg2) =
        string_literal(input).map_err(|i| ParseError::InvalidStringLiteral(i.to_owned()))?;
    let input = ignore_whitespace(input);
    if is_empty_or_comment(input) && cmd == "bind" {
        return Ok(Some(ConfigItem::Bind(arg1.to_owned(), arg2.to_owned())));
    }

    Err(ParseError::UnexpectedEndOfLine(input.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_parsing() -> Result<(), ParseError> {
        assert_eq!(
            parse_line(r#"bind "enter" "slot1""#)?,
            Some(ConfigItem::Bind("enter".to_owned(), "slot1".to_owned()))
        );
        assert_eq!(
            parse_line(r#"bind"mouse1""+attack""#)?,
            Some(ConfigItem::Bind("mouse1".to_owned(), "+attack".to_owned()))
        );
        assert_eq!(
            parse_line(r#"  bind    "4" "slot4"     // Comment  "#)?,
            Some(ConfigItem::Bind("4".to_owned(), "slot4".to_owned()))
        );
        assert!(parse_line(r#"bind "a" "non-ending string"#).is_err(),);

        Ok(())
    }

    #[test]
    fn test_cvar_parsing() -> Result<(), ParseError> {
        assert_eq!(
            parse_line(r#"sensitivity "1.5""#)?,
            Some(ConfigItem::Cvar("sensitivity".to_owned(), "1.5".to_owned()))
        );
        assert_eq!(
            parse_line(r#"   volume     "0.5"  // Comment here  "#)?,
            Some(ConfigItem::Cvar("volume".to_owned(), "0.5".to_owned()))
        );
        assert!(parse_line(r#"hud_scaling 0.8"#).is_err());

        Ok(())
    }

    #[test]
    fn test_cmd_parsing() -> Result<(), ParseError> {
        assert_eq!(
            parse_line(r#"  unbindall    "#)?,
            Some(ConfigItem::Command("unbindall".to_owned()))
        );
        assert_eq!(
            parse_line(r#"disconnect   //Comment Foo  "#)?,
            Some(ConfigItem::Command("disconnect".to_owned()))
        );
        assert!(parse_line(r#"1quit"#).is_err());

        Ok(())
    }
}
