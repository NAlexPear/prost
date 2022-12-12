use nom::{
    bytes::complete::tag,
    character::{complete::multispace0, streaming::alpha1},
    combinator::map,
    multi::many1,
    sequence::{delimited, terminated, tuple},
    IResult,
};
use prost_types::{EnumDescriptorProto, EnumValueDescriptorProto};

/// Parse an enum into an [`EnumDescriptorProto`]
pub(crate) fn parse(input: &str) -> IResult<&str, EnumDescriptorProto> {
    let (input, (_, _, _, name, _)) =
        tuple((multispace0, tag("enum"), multispace0, alpha1, multispace0))(input)?;

    let (input, values) = delimited(
        tag("{"),
        many1(map(
            tuple((
                delimited(multispace0, alpha1, multispace0),
                tag("="),
                delimited(multispace0, nom::character::complete::i32, multispace0),
                terminated(tag(";"), multispace0),
            )),
            |(name, _, number, _): (&str, _, _, _)| {
                EnumValueDescriptorProto {
                    name: Some(name.to_string()),
                    number: Some(number),
                    // FIXME: handle enum options, too
                    ..Default::default()
                }
            },
        )),
        tag("}"),
    )(input)?;

    Ok((
        input,
        EnumDescriptorProto {
            name: Some(name.to_string()),
            value: values,
            ..Default::default()
        },
    ))
}

#[cfg(test)]
mod test {
    use prost_types::{EnumDescriptorProto, EnumValueDescriptorProto};

    #[test]
    fn parses_valid_enum() {
        let name = "Testing".to_string();
        let first = "FIRST".to_string();
        let second = "SECOND".to_string();
        let input = format!(
            r#"enum {name} {{
                   {first} = 0;
                   {second} = 1;
               }}"#
        );
        let enum_type = EnumDescriptorProto {
            name: Some(name),
            value: vec![
                EnumValueDescriptorProto {
                    name: Some(first),
                    number: Some(0),
                    ..Default::default()
                },
                EnumValueDescriptorProto {
                    name: Some(second),
                    number: Some(1),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let (_, result) = super::parse(&input).unwrap();

        assert_eq!(enum_type, result);
    }
}
