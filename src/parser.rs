use crate::token::{Component, Condition, StyleToken, Token};
use anyhow::Result;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, multispace0, none_of};
use nom::combinator::{map, map_res, opt, verify};
use nom::multi::{many0, many1};
use nom::sequence::{pair, preceded, separated_pair, terminated, tuple};
use nom::IResult;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

static RESERVED_KEYWORDS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("end".to_string());
    s.insert("else".to_string());
    s
});

fn any_char_except_opening_brace(input: &str) -> IResult<&str, Token> {
    map(many1(none_of("{")), |s| Token::Static(String::from_iter(s)))(input)
}

fn escaped_opening_brace(input: &str) -> IResult<&str, Token> {
    map(tag("{{"), |s: &str| Token::Static(s.to_owned()))(input)
}

fn start_identifier_end(input: &str) -> IResult<&str, String> {
    terminated(preceded(tag("{"), identifier), tag("}"))(input)
}

fn style_token(input: &str) -> IResult<&str, StyleToken> {
    map_res(start_identifier_end, |s: String| s.parse::<StyleToken>())(input)
}

fn style(input: &str) -> IResult<&str, Token> {
    map(style_token, |s| Token::Style(s))(input)
}

// TODO: Handle spaces
fn if_start(input: &str) -> IResult<&str, ()> {
    map(pair(tag("{"), preceded(multispace0, tag("if"))), |_| ())(input)
}

fn if_condition(input: &str) -> IResult<&str, Condition> {
    terminated(
        preceded(
            multispace0,
            map_res(identifier, |s: String| s.parse::<Condition>()),
        ),
        tag("}"),
    )(input)
}

fn end(input: &str) -> IResult<&str, ()> {
    map(tag("{end}"), |_| ())(input)
}

fn if_else_tag(input: &str) -> IResult<&str, ()> {
    map(tag("{else}"), |_| ())(input)
}

fn if_else_branch(input: &str) -> IResult<&str, Vec<Token>> {
    preceded(if_else_tag, tokens)(input)
}

fn conditional(input: &str) -> IResult<&str, Token> {
    map(
        tuple((
            // {if condition}
            preceded(if_start, if_condition),
            // foo bar baz
            tokens,
            terminated(
                // {else}
                // foo bar baz
                opt(if_else_branch),
                // {end}
                end,
            ),
        )),
        |(condition, left, right)| Token::Conditional {
            condition,
            left,
            right,
        },
    )(input)
}

fn key(input: &str) -> IResult<&str, String> {
    map(preceded(multispace0, many1(none_of("="))), |s| {
        String::from_iter(s)
    })(input)
}

fn value(input: &str) -> IResult<&str, String> {
    map(many1(none_of("} ")), |s| String::from_iter(s))(input)
}

fn key_value(input: &str) -> IResult<&str, (String, String)> {
    separated_pair(key, tag("="), value)(input)
}

fn key_values(input: &str) -> IResult<&str, HashMap<String, String>> {
    map(many0(key_value), |options| options.into_iter().collect())(input)
}

fn underscore(input: &str) -> IResult<&str, &str> {
    tag("_")(input)
}

fn identifier(input: &str) -> IResult<&str, String> {
    verify(
        map(many1(alt((alpha1, underscore))), |s| String::from_iter(s)),
        |s: &str| !RESERVED_KEYWORDS.contains(s),
    )(input)
}

fn component(input: &str) -> IResult<&str, Token> {
    map(
        tuple((
            preceded(
                tag("{"),
                preceded(
                    multispace0,
                    map_res(identifier, |s: String| s.parse::<Component>()),
                ),
            ),
            terminated(key_values, preceded(multispace0, tag("}"))),
        )),
        |(name, options)| Token::Component { name, options },
    )(input)
}

fn tokens(input: &str) -> IResult<&str, Vec<Token>> {
    many1(alt((
        any_char_except_opening_brace,
        escaped_opening_brace,
        style,
        conditional,
        component,
    )))(input)
}

pub fn parse(input: &str) -> Result<Vec<Token>> {
    if let Ok((input, tokens)) = tokens(input) {
        if input.is_empty() {
            return Ok(tokens);
        }
    }
    Err(anyhow::anyhow!("parse error: {}", input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn it_parses_a_component() {
        assert_eq!(
            parse(&"{cwd}").unwrap(),
            vec![Token::Component {
                name: Component::Cwd,
                options: HashMap::new(),
            }]
        );
    }

    #[test]
    fn it_parses_a_component_and_static() {
        assert_eq!(
            parse(&"{cwd} $").unwrap(),
            vec![
                Token::Component {
                    name: Component::Cwd,
                    options: HashMap::new(),
                },
                Token::Static(" $".to_string())
            ]
        );
    }

    #[test]
    fn it_parses_a_component_with_options() {
        let mut options = HashMap::new();
        options.insert("style".to_string(), "default".to_string());
        assert_eq!(
            parse(&"{cwd style=default}").unwrap(),
            vec![Token::Component {
                name: Component::Cwd,
                options,
            },]
        );

        let mut options = HashMap::new();
        options.insert("style".to_string(), "short".to_string());
        assert_eq!(
            parse(&"{cwd style=short}").unwrap(),
            vec![Token::Component {
                name: Component::Cwd,
                options,
            },]
        );

        let mut options = HashMap::new();
        options.insert("style".to_string(), "long".to_string());
        assert_eq!(
            parse(&"{cwd style=long}").unwrap(),
            vec![Token::Component {
                name: Component::Cwd,
                options,
            },]
        );
    }

    #[test]
    fn it_parses_identifiers() {
        assert_eq!(identifier(&"cwd").unwrap().1, "cwd".to_string());
        assert_eq!(
            identifier(&"git_branch").unwrap().1,
            "git_branch".to_string()
        );

        assert!(identifier(&"end").is_err());
    }

    #[test]
    fn it_allows_escaped_braces_as_char() {
        assert_eq!(
            parse(&"{{cwd").unwrap(),
            vec![
                Token::Static("{{".to_string()),
                Token::Static("cwd".to_string()),
            ]
        );

        assert_eq!(
            parse(&"{{cwd{cwd}").unwrap(),
            vec![
                Token::Static("{{".to_string()),
                Token::Static("cwd".to_string()),
                Token::Component {
                    name: Component::Cwd,
                    options: HashMap::new(),
                }
            ]
        );
    }

    #[test]
    fn it_parses_components_and_options_with_spaces() {
        let mut options = HashMap::new();
        options.insert("a".to_string(), "bc".to_string());
        options.insert("d".to_string(), "12".to_string());

        assert_eq!(
            parse(&"{  git_branch a=bc   d=12  }  { git_commit }").unwrap(),
            vec![
                Token::Component {
                    name: Component::GitBranch,
                    options,
                },
                Token::Static("  ".to_string()),
                Token::Component {
                    name: Component::GitCommit,
                    options: HashMap::new(),
                },
            ]
        );
    }

    #[test]
    fn it_parses_options() {
        let mut options = HashMap::new();
        options.insert("a".to_string(), "bc".to_string());
        options.insert("d".to_string(), "12".to_string());

        assert_eq!(
            parse(&"{git_branch a=bc d=12}").unwrap(),
            vec![Token::Component {
                name: Component::GitBranch,
                options,
            }]
        );
    }

    #[test]
    fn it_parses_conditionals() {
        assert_eq!(
            parse(&"{if last_command_status}left{end}").unwrap(),
            vec![Token::Conditional {
                condition: Condition::LastCommandStatus,
                left: vec![Token::Static("left".to_string())],
                right: None,
            }]
        );
    }

    #[test]
    fn it_parses_conditionals_with_else_branch() {
        assert_eq!(
            parse(&"{if last_command_status}left{else}right{end}").unwrap(),
            vec![Token::Conditional {
                condition: Condition::LastCommandStatus,
                left: vec![Token::Static("left".to_string())],
                right: Some(vec![Token::Static("right".to_string())]),
            }]
        );
    }
}
