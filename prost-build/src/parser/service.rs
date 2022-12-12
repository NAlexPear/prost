use nom::{
    bytes::complete::{tag, take_till1},
    character::complete::{alpha1, multispace0, multispace1},
    multi::many0,
    sequence::{delimited, pair, terminated, tuple},
    IResult,
};
use prost_types::{MethodDescriptorProto, ServiceDescriptorProto};

/// Parse an rpc's input and output types from between parens
fn parse_parenthetical(input: &str) -> IResult<&str, &str> {
    delimited(
        pair(tag("("), multispace0),
        // FIXME: apply the rules for message names here
        take_till1(|character: char| character == ')' || character.is_whitespace()),
        pair(multispace0, tag(")")),
    )(input)
}

/// Parse an rpc into a [`MethodDescriptorProto`]
fn parse_method(input: &str) -> IResult<&str, MethodDescriptorProto> {
    let (input, (_, name, input_type, _, output_type, _)) = tuple((
        tag("rpc"),
        delimited(multispace1, alpha1, multispace0),
        parse_parenthetical,
        delimited(multispace1, tag("returns"), multispace1),
        parse_parenthetical,
        pair(multispace0, tag(";")),
    ))(input)?;

    Ok((
        input,
        MethodDescriptorProto {
            name: Some(name.to_string()),
            input_type: Some(input_type.to_string()),
            output_type: Some(output_type.to_string()),
            ..Default::default()
        },
    ))
}

/// Parse a service into a [`ServiceDescriptorProto`]
pub(crate) fn parse(input: &str) -> IResult<&str, ServiceDescriptorProto> {
    let (input, (_, name, methods)) = tuple((
        delimited(multispace0, tag("service"), multispace1),
        terminated(alpha1, multispace0),
        delimited(
            pair(tag("{"), multispace0),
            many0(parse_method),
            pair(multispace0, tag("}")),
        ),
    ))(input)?;

    Ok((
        input,
        ServiceDescriptorProto {
            name: Some(name.to_string()),
            method: methods,
            // FIXME: handle the rest of these fields, too
            ..Default::default()
        },
    ))
}

#[cfg(test)]
mod test {
    use prost_types::{MethodDescriptorProto, ServiceDescriptorProto};

    #[test]
    fn parses_valid_service() {
        let name = "Test".to_string();
        let method = "GetTest".to_string();
        let empty = "google.protobuf.Empty".to_string();
        let input = format!(
            r#"
            service {name} {{
                rpc {method} ({empty}) returns ({empty});
            }}
        "#
        );

        let service = ServiceDescriptorProto {
            name: Some(name),
            method: vec![MethodDescriptorProto {
                name: Some(method),
                input_type: Some(empty.clone()),
                output_type: Some(empty),
                ..Default::default()
            }],
            ..Default::default()
        };

        let (_, result) = super::parse(&input).unwrap();

        assert_eq!(service, result);
    }
}
