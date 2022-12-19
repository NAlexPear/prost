//! A protobuf file parser for generating a [`FileDescriptorProto`]. This parser is an alternative to
//! `protoc` for building [`FileDescriptorSet`]s.

use nom::combinator::all_consuming;
use prost_types::{DescriptorProto, FileDescriptorSet, SourceCodeInfo};
use source::{LocationRecorder, Span, State};
use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
};

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

/// Helper function for resolving message type paths across dependencies
fn resolve_message_type<'a>(
    type_name: &'a str,
    path: &'a Path,
    messages: &'a HashMap<PathBuf, Vec<String>>,
) -> Result<&'a str> {
    if type_name.starts_with(".") {
        // absolute path, check against other messages
        if messages
            .values()
            .flatten()
            .find(|message| message == &type_name)
            .is_none()
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("{type_name} not found in dependencies"),
            ));
        }

        Ok(type_name)
    } else {
        // relative path, check against the types in this package
        let messages = messages.get(path).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("{type_name} not found in dependencies"),
            )
        })?;

        let resolved_type_name = messages
            .iter()
            .find(|message| message.ends_with(type_name))
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("{type_name} not found in dependencies"),
                )
            })?;

        // FIXME: check against types in dependencies in the package, too!
        Ok(resolved_type_name)
    }
}
/// Parse a set of files into a [`FileDescriptorSet`]
pub(crate) fn parse(input: HashMap<PathBuf, (String, String)>) -> Result<FileDescriptorSet> {
    // generate the raw file descriptors
    let mut files = input
        .into_iter()
        .map(|(path, (name, input))| {
            // initialize internal parser state
            let locations = LocationRecorder::new();
            let state = State::new(&locations);
            let root_span = Span::new_extra(&input, state);

            // FIXME: handle errors more granularly through a shared custom type
            let (_, mut file_descriptor) =
                all_consuming(file::parse)(root_span).map_err(|error| {
                    Error::new(
                        ErrorKind::InvalidData,
                        format!("Error parsing proto file: {error}"),
                    )
                })?;

            // modify file_descriptor with global values
            file_descriptor.name = Some(name);
            file_descriptor.source_code_info = Some(SourceCodeInfo {
                location: locations.into_inner(),
            });

            Ok((path, file_descriptor))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    // create a hashmap of all of the fully-qualified message names in each file by absolute path
    let messages = files
        .iter()
        .map(|(path, file)| {
            let package = file.package();

            fn resolve_messages<'a>(package: &'a str, message: &'a DescriptorProto) -> Vec<String> {
                let name = message.name();
                // handle top-level message name
                let top_level_message = format!(".{package}.{name}");

                // handle nested messages
                let nested_messages = message
                    .nested_type
                    .iter()
                    .flat_map(|message| resolve_messages(package, message));

                // return the entire set of messages as a single iterator
                std::iter::once(top_level_message)
                    .chain(nested_messages)
                    .collect()
            }

            let message_types = file
                .message_type
                .iter()
                .flat_map(|message| resolve_messages(package, message))
                .collect::<Vec<_>>();

            (path.clone(), message_types)
        })
        .collect::<HashMap<_, _>>();

    // resolve relative type paths
    for (path, file) in files.iter_mut() {
        for service in file.service.iter_mut() {
            for method in service.method.iter_mut() {
                if let Some(input_type) = &method.input_type {
                    let resolved_input_type = resolve_message_type(input_type, path, &messages)?;
                    method.input_type = Some(resolved_input_type.to_string());
                }

                if let Some(output_type) = &method.output_type {
                    let resolved_output_type = resolve_message_type(output_type, path, &messages)?;
                    method.output_type = Some(resolved_output_type.to_string());
                }
            }
        }
    }

    Ok(FileDescriptorSet {
        file: files.into_values().collect(),
    })
}
