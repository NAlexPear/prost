use super::{
    comment, method,
    source::{Span, Tag},
};
use nom::{
    bytes::complete::{tag, take_until},
    character::complete::multispace0,
    multi::many0,
    sequence::{delimited, preceded},
    IResult,
};
use prost_types::{source_code_info::Location, ServiceDescriptorProto};

/// Path component for a [`Message`]
/// derived from the `service` field's tag in [`FileDescriptorProto`]
// FIXME: derive these tags directly from the FileDescriptorProto in prost_types
pub(crate) struct TAG;

impl Tag for TAG {
    fn into_path(&self, locations: &[Location]) -> Vec<i32> {
        let tag: i32 = self.into();

        // service paths are indexed, since they're repeating fields
        let next_service_index = locations
            .iter()
            .rev()
            .find(|location| location.path.get(0) == Some(&tag))
            .map(|location| location.path[1] + 1)
            .unwrap_or(0);

        vec![tag, next_service_index]
    }
}

impl<'a> From<&'a TAG> for i32 {
    fn from(_: &'a TAG) -> Self {
        6
    }
}

mod identifier {
    use super::*;

    /// Path component for any message with a `name` tag
    #[derive(Clone, Copy)]
    pub(crate) struct TAG;

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

/// Parse a service into a [`ServiceDescriptorProto`]
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, ServiceDescriptorProto> {
    // extract the service-level comments
    // FIXME: parse these comments into leading + leading_detached
    let (input, _) = many0(comment::parse)(input)?;

    // consume the input up the start of the service definition
    let (start, _) = take_until("service")(input)?;

    // start recording the syntax statement's location
    let location_record = input.extra.record_location_start(start, TAG);

    // extract the identifier
    let (input, identifier) =
        preceded(tag("service"), super::identifier::parse_as(identifier::TAG))(start)?;

    // consume methods until the service is finished
    let (end, methods) = delimited(
        tag("{"),
        many0(method::parse),
        preceded(multispace0, tag("}")),
    )(input)?;

    // finish recording the location
    input.extra.record_location_end(location_record, end);

    Ok((
        end,
        ServiceDescriptorProto {
            name: Some(identifier.to_string()),
            method: methods,
            // FIXME: handle the rest
            ..Default::default()
        },
    ))
}

#[cfg(test)]
mod test {
    use crate::parser::{
        source::{LocationRecorder, State},
        Span,
    };
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

        let locations = LocationRecorder::new();
        let state = State::new(&locations);
        let span = Span::new_extra(&input, state);

        let expected = ServiceDescriptorProto {
            name: Some(name),
            method: vec![MethodDescriptorProto {
                name: Some(method),
                input_type: Some(empty.clone()),
                output_type: Some(empty),
                ..Default::default()
            }],
            ..Default::default()
        };

        let (_, actual) = super::parse(span).unwrap();

        assert_eq!(expected, actual);
    }
}
