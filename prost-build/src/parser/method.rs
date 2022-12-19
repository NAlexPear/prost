use super::source::{locate, Span, Tag};
use nom::{
    bytes::complete::{tag, take_until},
    character::complete::{multispace0, multispace1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};
use prost_types::{source_code_info::Location, MethodDescriptorProto};

/// Path component for a [`Method`]
/// derived from the `method` field's tag in [`ServiceDescriptorProto`]
// FIXME: derive these tags directly from the ServiceDescriptorProto in prost_types
#[derive(Clone, Copy)]
pub(crate) struct TAG;

impl Tag for TAG {
    fn into_path(&self, locations: &[Location]) -> Vec<i32> {
        // methods are always attached to parent services
        let parent = locations.iter().last().unwrap(); // FIXME

        // parents should have at least three path components by this point
        assert!(parent.path.len() >= 3); // FIXME

        // figure out how to handle the tag based on parent path patterns
        match parent.path[..] {
            [6, _, 1] => {
                let mut path = parent.path.clone();
                path.pop();
                path.push(self.into());
                path.push(0);
                path
            }
            [6, _, 2, index] => {
                let mut path = parent.path.clone();
                path.pop();
                path.push(index + 1);
                path
            }
            _ => todo!("FIXME: failed to account for a path in {parent:?}"),
        }
    }
}

impl<'a> From<&'a TAG> for i32 {
    fn from(_: &'a TAG) -> Self {
        2
    }
}

mod identifier {
    use super::*;

    /// Path component for any message with a `name` tag
    #[derive(Clone, Copy)]
    pub(crate) struct TAG;

    impl Tag for TAG {
        fn into_path(&self, locations: &[Location]) -> Vec<i32> {
            // identifiers are always attached directly to parents
            let parent = locations.iter().last().unwrap(); // FIXME: make fallible
            let mut path = parent.path.clone();
            path.push(self.into());
            path
        }
    }

    impl<'a> From<&'a TAG> for i32 {
        fn from(_: &'a TAG) -> Self {
            1
        }
    }
}

mod input_type {
    use super::*;

    /// Path component for a method's input_type
    #[derive(Clone, Copy)]
    pub(crate) struct TAG;

    impl Tag for TAG {
        fn into_path(&self, locations: &[Location]) -> Vec<i32> {
            // input_types are always attached directly to parents after identifiers
            let parent = locations.iter().last().unwrap(); // FIXME: make fallible
            let mut path = parent.path.clone();
            path.pop();
            path.push(self.into());
            path
        }
    }

    impl<'a> From<&'a TAG> for i32 {
        fn from(_: &'a TAG) -> Self {
            2
        }
    }
}

mod output_type {
    use super::*;

    /// Path component for a method's output_type
    #[derive(Clone, Copy)]
    pub(crate) struct TAG;

    impl Tag for TAG {
        fn into_path(&self, locations: &[Location]) -> Vec<i32> {
            // output_types are always attached directly to parents after input_types
            let parent = locations.iter().last().unwrap(); // FIXME: make fallible
            let mut path = parent.path.clone();
            path.pop();
            path.push(self.into());
            path
        }
    }

    impl<'a> From<&'a TAG> for i32 {
        fn from(_: &'a TAG) -> Self {
            3
        }
    }
}

/// Parse an rpc into a [`Method`]
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, MethodDescriptorProto> {
    // FIXME: handle comments, whitespace, and location registration

    // consume the input up to the start of the rpc definition
    let (start, _) = take_until("rpc")(input)?;

    locate(
        |input| {
            // extract the rpc Identifier
            let (input, identifier) =
                preceded(tag("rpc"), super::identifier::parse_as(identifier::TAG))(input)?;

            // extract the input and output types
            let (end, (input_type, output_type)) = tuple((
                terminated(
                    delimited(
                        tag("("),
                        super::identifier::parse_as(input_type::TAG),
                        tag(")"),
                    ),
                    delimited(multispace1, tag("returns"), multispace1),
                ),
                terminated(
                    delimited(
                        tag("("),
                        super::identifier::parse_as(output_type::TAG),
                        tag(")"),
                    ),
                    pair(multispace0, tag(";")),
                ),
            ))(input)?;

            Ok((
                end,
                MethodDescriptorProto {
                    name: Some(identifier.to_string()),
                    input_type: Some(input_type.to_string()),
                    output_type: Some(output_type.to_string()),
                    ..Default::default()
                },
            ))
        },
        TAG,
    )(start)
}
