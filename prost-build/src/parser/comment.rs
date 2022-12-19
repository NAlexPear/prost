use super::Span;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{multispace0, not_line_ending},
    sequence::{delimited, pair, preceded},
    IResult,
};

/// Parse a comment
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, &'a str> {
    let (input, comment) = preceded(
        multispace0,
        alt((
            preceded(tag("//"), not_line_ending),
            delimited(pair(tag("/*"), multispace0), take_until("*/"), tag("*/")),
        )),
    )(input)?;

    Ok((input, &comment))
}

#[cfg(test)]
mod test {
    use nom::multi::many0;

    use crate::parser::source::{LocationRecorder, Span, State};

    #[test]
    fn parses_slash_style_line_comment() {
        let comment = "Testing testing 123".to_string();
        let input = format!(r#"//{comment}"#);
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

    #[test]
    fn parses_attached_comment() {
        let comment = "Testing attached comment".to_string();
        let input = format!(
            r#"
            //{comment}
            message Foo {{}}
            "#
        );
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (rest, result) = super::parse(span).unwrap();

        assert!(!rest.is_empty());
        assert_eq!(comment, result);
    }

    #[test]
    fn parses_detached_comment() {
        let comment = "Testing detached comment".to_string();
        let input = format!(
            r#"
            //{comment}

            message Foo {{}}
            "#
        );
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (rest, result) = super::parse(span).unwrap();

        assert!(!rest.is_empty());
        assert_eq!(comment, result);
    }

    #[test]
    fn parses_multiple_comments() {
        let comment = "Testing comments of all kind".to_string();
        let input = format!(
            r#"
            //{comment}

            //{comment}

            //{comment}
            message Foo {{}}
            "#
        );
        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);
        let (rest, result) = many0(super::parse)(span).unwrap();

        assert!(!rest.is_empty());
        assert_eq!(vec![comment.clone(), comment.clone(), comment], result);
    }
}
