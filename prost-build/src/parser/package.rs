use nom::{
    bytes::complete::{tag, take_till1},
    character::complete::multispace0,
    sequence::tuple,
    IResult,
};

/// Parse a package identifier
pub(crate) fn parse(input: &str) -> IResult<&str, String> {
    let (input, (_, _, _, package, _)) = tuple((
        multispace0,
        tag("package"),
        multispace0,
        take_till1(|character: char| character == ';' || character.is_whitespace()),
        tag(";"),
    ))(input)?;

    Ok((input, package.to_string()))
}

#[cfg(test)]
mod test {
    #[test]
    fn parses_valid_package() {
        let package = "testing.v1".to_string();
        let input = format!(r#"package {package};"#);
        let (_, result) = super::parse(&input).unwrap();

        assert_eq!(package, result);
    }
}
