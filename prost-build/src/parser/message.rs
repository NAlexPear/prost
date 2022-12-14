use super::{
    comment,
    source::{locate, Tag},
    Span,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1},
    character::complete::{alphanumeric1, multispace0, multispace1},
    combinator::{iterator, map, map_res},
    error::{Error, ErrorKind},
    multi::many0,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use prost_types::{
    field_descriptor_proto::Type, source_code_info::Location, DescriptorProto,
    FieldDescriptorProto, OneofDescriptorProto,
};

/// Path component for a [`Message`]
/// derived from the `message_type` field's tag in [`FileDescriptorProto`]
// FIXME: derive these tags directly from the FileDescriptorProto in prost_types
#[derive(Clone, Copy)]
struct TAG;

impl Tag for TAG {
    fn into_path(&self, locations: &[Location]) -> Vec<i32> {
        let tag: i32 = self.into();

        // message paths are indexed, since they're repeating fields
        let next_message_index = locations
            .iter()
            .rev()
            .find(|location| location.path.get(0) == Some(&tag))
            .map(|location| location.path[1] + 1)
            .unwrap_or(0);

        vec![tag, next_message_index]
    }
}

impl<'a> From<&'a TAG> for i32 {
    fn from(_: &'a TAG) -> Self {
        4
    }
}

/// Parse a message into a [`DescriptorProto`]
// FIXME: implement Parser<DescriptorProto> for FileDescriptorProto
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, DescriptorProto> {
    locate(
        |input| {
            // FIXME: handle comments throughout

            // extract the identifier
            let (input, identifier) = preceded(
                terminated(tag("message"), multispace1),
                super::identifier::parse_as(identifier::TAG),
            )(input)?;

            // create the placeholder protobuf
            let mut descriptor = DescriptorProto {
                name: Some(identifier.to_string()),
                ..Default::default()
            };

            // consume the opening statement bracket
            let (input, _) = tag("{")(input)?;

            // consume top-level statements until the message is finished
            let mut statements = iterator(
                input,
                alt((
                    map(field::parse, Statement::Field),
                    map(oneof::parse, Statement::OneOf),
                )),
            );

            // iterate over the statements
            for statement in &mut statements {
                match statement {
                    Statement::Field(field) => descriptor.field.push(field),
                    Statement::OneOf(oneof) => {
                        let oneof_index = descriptor.oneof_decl.len() as i32;
                        descriptor.oneof_decl.push(oneof.descriptor);

                        for mut field in oneof.fields {
                            field.oneof_index = Some(oneof_index);
                            descriptor.field.push(field);
                        }
                    }
                }
            }

            let (input, _) = statements.finish()?;

            // consume the closing statement bracket
            let (end, _) = preceded(multispace0, tag("}"))(input)?;

            Ok((end, descriptor))
        },
        TAG,
    )(input)
}

/// Supported top-level statements in a `message`
enum Statement {
    Field(FieldDescriptorProto),
    OneOf(oneof::OneOf),
    // FIXME: implement all of the other message fields
}

pub(crate) mod identifier {
    use super::*;

    /// Path component for any message with a `name` tag
    #[derive(Clone, Copy)]
    pub(super) struct TAG;

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

mod field {
    use super::*;

    /// Path component for a message field
    /// derived from the `field` field in [`DescriptorProto`];
    // FIXME: derive these tags directly from the DescriptorProto in prost_types
    pub(super) struct TAG;

    impl Tag for TAG {
        fn into_path(&self, locations: &[Location]) -> Vec<i32> {
            // fields are always attached to parents
            let parent = locations.iter().last().unwrap(); // FIXME

            // parents should have at least three path components by this point
            assert!(parent.path.len() >= 3); // FIXME

            // figure out how to handle the tag based on parent path patterns
            match parent.path[..] {
                [4, _, 1] => {
                    let mut path = parent.path.clone();
                    path.pop();
                    path.push(self.into());
                    path.push(0);
                    path
                }
                [4, _, 2, index] => {
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

    /// parse a single message field
    pub(super) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, FieldDescriptorProto> {
        // FIXME: handle comments throughout
        // FIXME: consume up to the start of the first alphanumeric
        let (start, _) = many0(tuple((comment::parse, multispace0)))(input)?;

        // start recording the field's location
        // FIXME: this way of recording locations doesn't allow for failure! we need to be able to
        // unwind the location stack (or do we just need to filter on COMPLETE locations?)
        let location_record = input.extra.record_location_start(start, TAG);

        // FIXME: divide these parsers up, recording locations more granularly
        let (end, field) = map(
            tuple((
                map_res(
                    delimited(multispace0, alphanumeric1, multispace0),
                    |type_: Span<'a>| {
                        // FIXME: handle possible field types with an alt() instead of this
                        let type_ = match type_.as_ref() {
                            "double" => Type::Double,
                            "float" => Type::Float,
                            "int64" => Type::Int64,
                            "uint64" => Type::Uint64,
                            "int32" => Type::Int32,
                            "fixed64" => Type::Fixed64,
                            "fixed32" => Type::Fixed32,
                            "bool" => Type::Bool,
                            "string" => Type::String,
                            "bytes" => Type::Bytes,
                            "uint32" => Type::Uint32,
                            "sfixed32" => Type::Sfixed32,
                            "sfixed64" => Type::Sfixed64,
                            "sint32" => Type::Sint32,
                            "sint64" => Type::Sint64,
                            _ => return Err(Error::new(input, ErrorKind::Fail)),
                        };

                        Ok(type_)
                    },
                ),
                delimited(
                    multispace0,
                    take_till1(|character: char| character.is_whitespace()),
                    multispace0,
                ),
                tag("="),
                delimited(multispace0, nom::character::complete::i32, multispace0),
                terminated(tag(";"), multispace0),
            )),
            |(type_, name, _, number, _): (_, Span<'a>, _, _, _)| {
                FieldDescriptorProto {
                    name: Some(name.to_string()),
                    number: Some(number),
                    r#type: Some(type_ as i32),
                    // FIXME: handle the rest of these fields, too
                    ..Default::default()
                }
            },
        )(start)?;

        // finish recording the field
        input.extra.record_location_end(location_record, end);

        Ok((end, field))
    }
}

mod oneof {
    use super::*;

    /// Custom OneOf type for storing the [`OneofDescriptorProto`] and all of its fields
    #[derive(Default)]
    pub(super) struct OneOf {
        pub(super) descriptor: OneofDescriptorProto,
        pub(super) fields: Vec<FieldDescriptorProto>,
    }

    pub(super) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, OneOf> {
        // FIXME: handle whitespace, comments, and location-recording

        // extract the OneOf properties that we need for later
        let (input, (name, fields)) = tuple((
            preceded(
                delimited(multispace0, tag("oneof"), multispace1),
                terminated(
                    // FIXME: enforce field naming conventions
                    take_till1(|character: char| character.is_whitespace()),
                    multispace0,
                ),
            ),
            terminated(
                // FIXME: verify if oneof members are always FIELDS or any STATEMENT
                delimited(tag("{"), many0(field::parse), tag("}")),
                multispace0,
            ),
        ))(input)?;

        let one_of = OneofDescriptorProto {
            name: Some(name.to_string()),
            ..Default::default()
        };

        Ok((
            input,
            OneOf {
                descriptor: one_of,
                fields,
            },
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::parser::{
        source::{LocationRecorder, State},
        Span,
    };
    use prost_types::{
        field_descriptor_proto::Type, DescriptorProto, FieldDescriptorProto, OneofDescriptorProto,
    };

    #[test]
    fn generates_correct_empty_message_paths() {
        let name = "Testing".to_string();
        let input = format!(
            r#"message {name} {{
               }}"#
        );

        let expected = vec![vec![4, 0], vec![4, 0, 1]];
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        super::parse(span).unwrap();

        let actual: Vec<_> = locations
            .into_inner()
            .into_iter()
            .map(|location| location.path)
            .collect();

        assert_eq!(expected, actual)
    }

    #[test]
    fn generates_message_field_paths() {
        let location_recorder = LocationRecorder::new();
        let state = State::new(&location_recorder);
        let span = Span::new_extra("", state);
        span.extra.record_location_start(span, ());

        let expected = vec![
            vec![],
            vec![Into::<i32>::into(&super::TAG), 0],
            vec![
                Into::<i32>::into(&super::TAG),
                0,
                Into::<i32>::into(&super::identifier::TAG),
            ],
        ];

        span.extra.record_location_start(span, super::TAG);
        span.extra
            .record_location_start(span, super::identifier::TAG);

        let actual: Vec<_> = span
            .extra
            .into_inner()
            .into_iter()
            .map(|location| location.path)
            .collect();

        assert_eq!(expected, actual)
    }

    #[test]
    fn parses_valid_scalar_message() {
        let name = "Testing".to_string();
        let first = "first_field".to_string();
        let second = "second_field".to_string();
        let input = format!(
            r#"message {name} {{
                   string {first} = 1;
                   int32 {second} = 2;
               }}"#
        );

        let expected = DescriptorProto {
            name: Some(name),
            field: vec![
                FieldDescriptorProto {
                    name: Some(first),
                    number: Some(1),
                    r#type: Some(Type::String as i32),
                    ..Default::default()
                },
                FieldDescriptorProto {
                    name: Some(second),
                    number: Some(2),
                    r#type: Some(Type::Int32 as i32),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (_, actual) = super::parse(span).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn parses_empty_message() {
        let name = "Testing".to_string();
        let input = format!(
            r#"message {name} {{
            }}"#
        );

        let expected = DescriptorProto {
            name: Some(name),
            ..Default::default()
        };

        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (_, actual) = super::parse(span).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn parses_valid_oneof_field() {
        let name = "Testing".to_string();
        let one_of = "test_oneof".to_string();
        let left = "name".to_string();
        let right = "id".to_string();
        let input = format!(
            r#"message {name} {{
                 oneof {one_of} {{
                     string {left} = 1;
                     int32 {right} = 2;
                 }}
               }}"#
        );

        let expected = DescriptorProto {
            name: Some(name),
            oneof_decl: vec![OneofDescriptorProto {
                name: Some(one_of),
                ..Default::default()
            }],
            field: vec![
                FieldDescriptorProto {
                    name: Some(left),
                    number: Some(1),
                    r#type: Some(Type::String as i32),
                    oneof_index: Some(0),
                    ..Default::default()
                },
                FieldDescriptorProto {
                    name: Some(right),
                    number: Some(2),
                    r#type: Some(Type::Int32 as i32),
                    oneof_index: Some(0),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (_, actual) = super::parse(span).unwrap();

        assert_eq!(expected, actual);
    }
}
