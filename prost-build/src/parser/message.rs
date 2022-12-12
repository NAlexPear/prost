use nom::{
    bytes::complete::{tag, take_till1},
    character::{
        complete::{alphanumeric1, multispace0},
        streaming::alpha1,
    },
    combinator::map,
    multi::many1,
    sequence::{delimited, terminated, tuple},
    IResult,
};
use prost_types::{field_descriptor_proto::Type, DescriptorProto, FieldDescriptorProto};

/// Parse a message into a [`DescriptorProto`]
pub(crate) fn parse(input: &str) -> IResult<&str, DescriptorProto> {
    let (input, (_, _, _, name, _)) = tuple((
        multispace0,
        tag("message"),
        multispace0,
        alpha1,
        multispace0,
    ))(input)?;

    let (input, fields) = delimited(
        tag("{"),
        many1(map(
            tuple((
                map(
                    delimited(multispace0, alphanumeric1, multispace0),
                    |type_| match type_ {
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
                        _ => todo!("handle Group, Enum, Message, and invalid types"),
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
        )),
        tag("}"),
    )(input)?;

    Ok((
        input,
        DescriptorProto {
            name: Some(name.to_string()),
            field: fields,
            ..Default::default()
        },
    ))
}

#[cfg(test)]
mod test {
    use prost_types::{field_descriptor_proto::Type, DescriptorProto, FieldDescriptorProto};

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
}
