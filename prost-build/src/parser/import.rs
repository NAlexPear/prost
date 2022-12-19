use super::{
    source::{locate, Tag},
    Span,
};
use nom::{
    bytes::complete::{tag, take_till1},
    character::complete::multispace1,
    combinator::map,
    sequence::{delimited, pair, preceded},
    IResult,
};
use prost_types::source_code_info::Location;
use std::fmt::{self, Display};

/// Path component for an [`Import`]
/// derived from the `dependency` field's tag in [`FileDescriptorProto`]
#[derive(Clone, Copy)]
struct TAG;

impl Tag for TAG {
    fn into_path(&self, _: &[Location]) -> Vec<i32> {
        vec![Into::<i32>::into(self)]
    }
}

impl<'a> From<&'a TAG> for i32 {
    fn from(_: &'a TAG) -> Self {
        3
    }
}

/// Convenience wrapper for imports
#[derive(Debug, PartialEq)]
pub(crate) struct Import<'a>(&'a str);

impl<'a> Import<'a> {
    fn new(inner: &'a str) -> Self {
        Self(inner)
    }
}

impl<'a> Display for Import<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Parse a standard (not weak or public) import statement/dependency
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, Import<'a>> {
    // FIXME: handle comments and weak/public dependencies

    // extract the import value
    locate(
        preceded(
            pair(tag("import"), multispace1),
            delimited(
                tag("\""),
                map(
                    take_till1(|character: char| character == '"' || character.is_whitespace()),
                    |import: Span<'a>| Import::new(&import),
                ),
                tag("\";"),
            ),
        ),
        TAG,
    )(input)
}

#[cfg(test)]
mod test {
    use super::Import;
    use crate::parser::source::{LocationRecorder, Span, State};

    #[test]
    fn parses_valid_import() {
        let import = Import::new("google/api/annotations.proto");
        let input = format!(r#"import "{import}";"#);
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (_, result) = super::parse(span).unwrap();

        assert_eq!(import, result);
    }
}
