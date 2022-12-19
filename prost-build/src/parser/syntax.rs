use super::{
    source::{locate, Tag},
    Span,
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::multispace1,
    combinator::value,
    sequence::{delimited, preceded, tuple},
    IResult,
};
use prost_types::source_code_info::Location;
use std::fmt::{self, Display};

/// Path component for a [`Syntax`]
/// derived from the `syntax` field's tag in [`FileDescriptorProto`]
#[derive(Clone, Copy)]
struct TAG;

impl Tag for TAG {
    fn into_path(&self, _: &[Location]) -> Vec<i32> {
        vec![Into::<i32>::into(self)]
    }
}

impl<'a> From<&'a TAG> for i32 {
    fn from(_: &'a TAG) -> Self {
        12
    }
}

/// All possible syntaxes supported by this parser
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Syntax {
    Proto2,
    Proto3,
}

/// Handle String conversions for appending to file descriptors
impl Display for Syntax {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Proto2 => formatter.write_str("proto2"),
            Self::Proto3 => formatter.write_str("proto3"),
        }
    }
}

/// Parse the file's required syntax statement (i.e. `proto2` or `proto3`)
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, Syntax> {
    locate(
        preceded(
            tuple((tag("syntax"), multispace1, tag("="), multispace1)),
            delimited(
                tag("\""),
                alt((
                    value(Syntax::Proto2, tag("proto2")),
                    value(Syntax::Proto3, tag("proto3")),
                )),
                tag("\";"),
            ),
        ),
        TAG,
    )(input)
}

#[cfg(test)]
mod test {
    use super::Syntax;
    use crate::parser::source::{LocationRecorder, Span, State};

    #[test]
    fn parses_valid_syntax() {
        let syntax = Syntax::Proto3;
        let input = format!(r#"syntax = "{syntax}";"#);
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (_, result) = super::parse(span).unwrap();

        assert_eq!(syntax, result);
    }
}
