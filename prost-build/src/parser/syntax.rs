use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::multispace0,
    sequence::{delimited, tuple},
    IResult,
};

/// Parse the file's required syntax statement (i.e. `proto2` or `proto3`)
pub(crate) fn parse(input: &str) -> IResult<&str, String> {
    let (input, (_, _, _, _, _, syntax)) = tuple((
        multispace0,
        tag("syntax"),
        multispace0,
        tag("="),
        multispace0,
        delimited(tag("\""), alt((tag("proto2"), tag("proto3"))), tag("\";")),
    ))(input)?;

    Ok((input, syntax.to_string()))
}

#[cfg(test)]
mod test {
    #[test]
    fn parses_valid_syntax() {
        let syntax = "proto3".to_string();
        let input = format!(r#"syntax = "{syntax}";"#);
        let (_, result) = super::parse(&input).unwrap();

        assert_eq!(syntax, result);
    }
}
