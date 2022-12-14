//! A protobuf file parser for generating a [`FileDescriptorProto`]. This parser is an alternative to
//! `protoc` for building [`FileDescriptorSet`]s.

use nom::combinator::all_consuming;
use prost_types::{FileDescriptorProto, SourceCodeInfo};
use source::{LocationRecorder, Span, State};
use std::io::{Error, ErrorKind, Result};

mod comment;
mod r#enum;
mod file;
mod identifier;
mod import;
mod message;
mod method;
mod package;
mod service;
mod source;
mod syntax;

/// Parse a file into a [`FileDescriptorProto`]
pub(crate) fn parse(input: &str) -> Result<FileDescriptorProto> {
    // initialize internal parser state
    let locations = LocationRecorder::new();
    let state = State::new(&locations);
    let root_span = Span::new_extra(input, state);

    // FIXME: handle errors more granularly through a shared custom type
    let (_, mut file_descriptor) = all_consuming(file::parse)(root_span).map_err(|error| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Error parsing proto file: {error}"),
        )
    })?;

    file_descriptor.source_code_info = Some(SourceCodeInfo {
        location: locations.into_inner(),
    });

    Ok(file_descriptor)
}
