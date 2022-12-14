use super::Span;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{multispace0, not_line_ending},
    sequence::{delimited, pair, preceded},
    IResult,
};

/// Parse a comment
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, String> {
    let (input, comment) = delimited(
        multispace0,
        alt((
            preceded(pair(tag("//"), multispace0), not_line_ending),
            delimited(pair(tag("/*"), multispace0), take_until("*/"), tag("*/")),
        )),
        multispace0,
    )(input)?;

    Ok((input, comment.to_string()))
}

#[cfg(test)]
mod test {
    use crate::parser::source::{LocationRecorder, Span, State};

    #[test]
    fn parses_slash_style_line_comment() {
        let comment = "Testing testing 123".to_string();
        let input = format!(r#"// {comment}"#);
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (_, result) = super::parse(span).unwrap();
        assert_eq!(comment, result);
    }

    #[test]
    fn parses_doc_style_line_comment() {
        let comment = "Testing testing 123".to_string();
        let input = format!(r#"/*{comment}*/"#);
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (_, result) = super::parse(span).unwrap();
        assert_eq!(comment, result);
    }
}
