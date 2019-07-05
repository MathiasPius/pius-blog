use tera::{Value, Result, to_value};
use std::collections::HashMap;
use combine::error::ParseError;
use combine::parser::{char::{char, letter, digit, string}, combinator::recognize};
use combine::{choice, any, skip_many, attempt, many1, one_of, none_of, optional, Parser, Stream};

#[derive(Debug)]
enum Type {
	Keyword,
	Name,
	Number,
	Whitespace,
	String,
	None
}

#[derive(Debug)]
struct Token {
	value: String,
	ttype: Type
}

impl Token {
	fn new<S: ToString>(ttype: Type, value: S) -> Token {
		Token {
			ttype,
			value: value.to_string()
		}
	}
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.ttype {
			Type::Keyword => write!(f, "<code class=\"hl-keyword\">{}</code>", tera::escape_html(&self.value)),
			Type::Name => write!(f, "<code class=\"hl-name\">{}</code>", tera::escape_html(&self.value)),
			Type::Number => write!(f, "<code class=\"hl-number\">{}</code>", tera::escape_html(&self.value)),
			Type::String => write!(f, "<code class=\"hl-quote\">{}</code>", tera::escape_html(&self.value)),
			_ => write!(f, "{}", tera::escape_html(&self.value))
		}
    }
}


fn keyword<'a, I>() -> impl Parser<Input = I, Output = Token>
    where I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>
{
    choice((
        attempt(string("if")), 
        attempt(string("else")), 
        attempt(string("fn")),
		attempt(string("impl")),
		attempt(string("where")),
		attempt(string("match")),
		attempt(string("let")),
		attempt(string("mut"))
    )).map(|v| Token { ttype: Type::Keyword, value: v.to_string()})
}


fn quoted_string<'a, I>() -> impl Parser<Input = I, Output = Token>
    where I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>
{
	recognize((char('\"'), skip_many(none_of("\"".chars())), char('\"')))
		.map(|v: String| Token::new(Type::String, v))
}


fn word<'a, I>() -> impl Parser<Input = I, Output = Token>
    where I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>
{
	recognize((letter(), skip_many(choice((letter(), digit(), char('_'))))))
		.map(|v: String| Token::new(Type::Name, v))
}


fn number<'a, I>() -> impl Parser<Input = I, Output = Token>
    where I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>
{
	recognize((digit(), skip_many(choice((digit(), char('.')))), optional(char('f'))))
		.map(|v: String| Token::new(Type::Number, v))
}


fn whitespace<'a, I>() -> impl Parser<Input = I, Output = Token>
    where I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>
{
	one_of("\t\r\n ".chars()).map(|v| Token::new(Type::Whitespace, v))
}


fn catch_all<'a, I>() -> impl Parser<Input = I, Output = Token>
    where I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>
{
	any().map(|v: char| Token::new(Type::None, v.to_string()))
}

fn root<'a, I>() -> impl Parser<Input = I, Output = Vec<Token>>
    where I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>
{
	many1(
		choice((
			attempt(keyword()),
			attempt(quoted_string()),
			attempt(number()),
			attempt(word()),
			attempt(whitespace()),
			catch_all()
		))
	)
}

pub fn highlight(value: &Value, _map: &HashMap<String, Value>) -> Result<Value> {
	match value.as_str() {
		Some(value) => {
			let tokens = root().parse(value);
			match tokens {
				Ok((tokens, _)) => {
					let mut concat = String::new();

					concat.push_str("<code>");
					for token in &tokens {
						concat.push_str(&format!("{}", token));
					}
					concat.push_str("</code>");

					Ok(to_value(concat).unwrap())
				},
				Err(_) => Ok(to_value("failed to parse code for syntax highlighting").unwrap())
			}
		},
		_ => Ok(to_value("value is not a string apparently").unwrap())
	}
}

pub fn codeblock(value: &Value, _map: &HashMap<String, Value>) -> Result<Value> {
	match value.as_str() {
		Some(value) => {
			let mut concat = String::new();

			concat.push_str("<pre>");
			for character in value.chars() {
				match character {
					'\t' => concat.push_str("&nbsp;&nbsp;&nbsp;&nbsp;"),
					'\n' => concat.push_str("<br />"),
					_ => concat.push(character)
				};
			}
			concat.push_str("</pre>");

			Ok(to_value(concat).unwrap())
		},
		_ => Ok(to_value("value is not a string apparently").unwrap())
	}
}
