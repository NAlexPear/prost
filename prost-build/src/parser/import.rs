use nom::{
    bytes::complete::{tag, take_till1},
    character::complete::multispace0,
    sequence::tuple,
    IResult,
};

/// Parse a standard (not weak or public) import statement/dependency
pub(crate) fn parse(input: &str) -> IResult<&str, String> {
    let (input, (_, _, _, _, import, _)) = tuple((
        multispace0,
        tag("import"),
        multispace0,
        tag("\""),
        take_till1(|character: char| character == '"' || character.is_whitespace()),
        tag("\";"),
    ))(input)?;

    Ok((input, import.to_string()))
}

#[cfg(test)]
mod test {
    #[test]
    fn parses_valid_import() {
        let import = "google/api/annotations.proto".to_string();
        let input = format!(r#"import "{import}";"#);
        let (_, result) = super::parse(&input).unwrap();

        assert_eq!(import, result);
    }
}
