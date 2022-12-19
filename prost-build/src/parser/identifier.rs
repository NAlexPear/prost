use super::{
    source::{locate, Tag},
    Span,
};
use nom::{bytes::complete::take_till1, combinator::map, IResult};
use std::fmt::{self, Display};

/// Convenience wrapper for an identifier (e.g. message names or rpc input_types)
pub(crate) struct Identifier<'a>(&'a str);

impl<'a> Identifier<'a> {
    fn new(inner: Span<'a>) -> Self {
        Self(&inner)
    }
}

impl<'a> Display for Identifier<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Parse an input into an [`Identifier`] (e.g. [`Message`] names or [`Method`] input types)
fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, Identifier<'a>> {
    // extract and verify the Identifier
    // FIXME: use verify() to enforce message ident naming conventions
    map(
        take_till1(|character: char| !(character.is_alphabetic() || character == '.')),
        Identifier::new,
    )(input)
}

/// Wrapper function for generating a located identifier parser with a Tag
pub(crate) fn parse_as<'a, T>(tag: T) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, Identifier<'a>>
where
    T: Tag + Copy,
{
    locate(parse, tag)
}
