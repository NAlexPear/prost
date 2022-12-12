use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1},
    character::{
        complete::{alphanumeric1, multispace0, multispace1},
        streaming::alpha1,
    },
    combinator::{map, map_res},
    error::{Error, ErrorKind},
    multi::many1,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use prost_types::{
    descriptor_proto::{ExtensionRange, ReservedRange},
    field_descriptor_proto::Type,
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, MessageOptions,
    OneofDescriptorProto,
};

// possible message-level statements
#[derive(Debug)]
enum Statement {
    Message(DescriptorProto),
    Enum(EnumDescriptorProto),
    Extensions(ExtensionRange),
    Reserved(ReservedRange),
    Extend(FieldDescriptorProto),
    Option(MessageOptions),
    OneOf(OneOf),
    Field(FieldDescriptorProto),
}

/// Custom OneOf type for storing the [`OneofDescriptorProto`] and all of its fields
#[derive(Debug)]
struct OneOf {
    descriptor: OneofDescriptorProto,
    fields: Vec<FieldDescriptorProto>,
}

/// Parse a single message field into a [`FieldDescriptorProto`]
fn parse_field(input: &str) -> IResult<&str, FieldDescriptorProto> {
    let (input, field) = map(
        tuple((
            map_res(
                delimited(multispace0, alphanumeric1, multispace0),
                |type_| {
                    let type_ = match type_ {
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
        |(type_, name, _, number, _): (_, &str, _, _, _)| {
            FieldDescriptorProto {
                name: Some(name.to_string()),
                number: Some(number),
                r#type: Some(type_ as i32),
                // FIXME: handle the rest of these fields, too
                ..Default::default()
            }
        },
    )(input)?;

    Ok((input, field))
}

/// Parse a single message `OneOf` "field" into a [`OneofDescriptorProto`]
fn parse_oneof(input: &str) -> IResult<&str, OneOf> {
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
            // FIXME: figure out if oneof members are FIELDS or STATEMENTs
            delimited(tag("{"), many1(parse_field), tag("}")),
            multispace0,
        ),
    ))(input)?;

    let one_of = OneOf {
        descriptor: OneofDescriptorProto {
            name: Some(name.to_string()),
            ..Default::default()
        },
        fields,
    };

    Ok((input, one_of))
}

/// Parse a message into a [`DescriptorProto`]
pub(crate) fn parse(input: &str) -> IResult<&str, DescriptorProto> {
    let (input, (name, statements)) = tuple((
        preceded(
            delimited(multispace0, tag("message"), multispace1),
            // FIXME: enforce message naming conventions
            terminated(alpha1, multispace0),
        ),
        delimited(
            tag("{"),
            many1(alt((
                map(parse_field, Statement::Field),
                map(parse_oneof, Statement::OneOf),
            ))),
            tag("}"),
        ),
    ))(input)?;

    let mut descriptor = DescriptorProto {
        name: Some(name.to_string()),
        ..Default::default()
    };

    for statement in statements {
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
            unsupported => todo!("fixme: implement {unsupported:?}"),
        }
    }

    Ok((input, descriptor))
}

#[cfg(test)]
mod test {
    use prost_types::{
        field_descriptor_proto::Type, DescriptorProto, FieldDescriptorProto, OneofDescriptorProto,
    };

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

        let message = DescriptorProto {
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

        let (_, result) = super::parse(&input).unwrap();

        assert_eq!(message, result);
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

        let message = DescriptorProto {
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

        let (_, result) = super::parse(&input).unwrap();

        assert_eq!(message, result);
    }
}
