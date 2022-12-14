use super::{
    source::{locate, Tag},
    Span,
};
use nom::{
    bytes::complete::{tag, take_till1, take_until},
    character::complete::{multispace0, multispace1},
    combinator::map,
    sequence::{delimited, tuple},
    IResult,
};
use prost_types::source_code_info::Location;
use std::fmt::{self, Display};

/// Path component for a [`Package`]
/// derived from the `package` field's tag in [`FileDescriptorProto`]
#[derive(Clone, Copy)]
struct TAG;

impl Tag for TAG {
    fn into_path(&self, _: &[Location]) -> Vec<i32> {
        vec![Into::<i32>::into(self)]
    }
}

impl<'a> From<&'a TAG> for i32 {
    fn from(_: &'a TAG) -> Self {
        2
    }
}

/// Convenience wrapper for package identifiers
#[derive(Debug, PartialEq)]
pub(crate) struct Package<'a>(&'a str);

impl<'a> Package<'a> {
    fn new(inner: &'a str) -> Self {
        Self(inner)
    }
}

impl<'a> Display for Package<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Parse a package identifier
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, Package<'a>> {
    locate(
        |input| {
            // FIXME: handle comments throughout

            // consume the input up the start of the package definition
            let (start, _) = take_until("package")(input)?;

            // extract the package itself
            let (end, package) = delimited(
                tuple((tag("package"), multispace1)),
                // FIXME: enforce/verify package naming conventions
                map(
                    take_till1(|character: char| character == ';' || character.is_whitespace()),
                    |package: Span<'a>| Package::new(&package),
                ),
                tuple((multispace0, tag(";"))),
            )(start)?;

            Ok((end, package))
        },
        TAG,
    )(input)
}

#[cfg(test)]
mod test {
    use super::Package;
    use crate::parser::source::{LocationRecorder, Span, State};

    #[test]
    fn parses_valid_package() {
        let package = Package::new("testing.v1");
        let input = format!(r#"package {package};"#);
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (_, result) = super::parse(span).unwrap();

        assert_eq!(package, result);
    }
}
