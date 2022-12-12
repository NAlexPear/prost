//! A protobuf file parser for generating a [`FileDescriptorProto`]. This parser is an alternative to
//! `protoc` for building [`FileDescriptorSet`]s.
#![allow(dead_code)] // FIXME

use nom::{branch::alt, combinator::map, IResult};
use prost_types::{
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileOptions,
    ServiceDescriptorProto,
};
use std::io::{Error, ErrorKind, Result};

mod r#enum;
mod import;
mod message;
mod package;
mod service;
mod syntax;

/// Allowed top-level statements within a file
enum Statement {
    Enum(EnumDescriptorProto),
    Extend(FieldDescriptorProto),
    Import(String),
    Message(DescriptorProto),
    Option(FileOptions),
    Package(String),
    Service(ServiceDescriptorProto),
}

impl Statement {
    fn add_to_file_descriptor(self, file_descriptor: &mut FileDescriptorProto) -> Result<()> {
        match self {
            Self::Enum(enum_type) => file_descriptor.enum_type.push(enum_type),
            Self::Extend(extension) => file_descriptor.extension.push(extension),
            Self::Import(dependency) => file_descriptor.dependency.push(dependency),
            Self::Message(message_type) => file_descriptor.message_type.push(message_type),
            Self::Service(service) => file_descriptor.service.push(service),
            Self::Package(package) => {
                if file_descriptor.package.is_none() {
                    file_descriptor.package.replace(package);
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Multiple package definitions.",
                    ));
                }
            }
            // FIXME: check correct behavior for duplicates
            Self::Option(option) => {
                file_descriptor.options.replace(option);
            }
        }

        Ok(())
    }
}

/// Parse any of the valid top-level statements found in a file
fn parse_top_level_statement(input: &str) -> IResult<&str, Statement> {
    let (input, statement) = alt((
        map(package::parse, Statement::Package),
        map(import::parse, Statement::Import),
        map(r#enum::parse, Statement::Enum),
        map(message::parse, Statement::Message),
        map(service::parse, Statement::Service),
    ))(input)?;

    Ok((input, statement))
}

/// Parse a file into a [`FileDescriptorProto`]
pub(crate) fn parse(input: &str) -> Result<FileDescriptorProto> {
    // FIXME: handle location/source info
    let (mut input, syntax) = syntax::parse(input.trim()).map_err(|_| Error::new(
        ErrorKind::InvalidData,
        r#"Error parsing proto syntax. File must begin with a valid syntax statement, e.g. 'syntax = "proto2";'."#
    ))?;

    let mut file_descriptor = FileDescriptorProto {
        syntax: Some(syntax),
        ..Default::default()
    };

    // consume top-level statements until the file is finished
    while !input.is_empty() {
        // FIXME: improve and specialize this error rather than yeeting nom errors up
        let (rest, statement) = parse_top_level_statement(input)
            .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;

        statement.add_to_file_descriptor(&mut file_descriptor)?;

        input = rest;
    }

    Ok(file_descriptor)
}

#[cfg(test)]
mod test {
    use prost_types::{
        field_descriptor_proto::Type, DescriptorProto, FieldDescriptorProto, FileDescriptorProto,
        MethodDescriptorProto, ServiceDescriptorProto,
    };

    #[test]
    fn parses_valid_file() {
        let syntax = "proto3".to_string();
        let package = "test.v1".to_string();
        let annotations_import = "google/api/annotations.proto".to_string();
        let empty_import = "google/protobuf/empty.proto".to_string();
        let foo_import = "foo.proto".to_string();
        let service = "Test".to_string();
        let method = format!("Get{service}");
        let message = format!("{method}Response");
        let empty_message = "google.protobuf.Empty".to_string();
        let input = format!(
            r#"syntax = "{syntax}";
               package {package};

               import "{annotations_import}";
               import "{empty_import}";
               import "{foo_import}";

               service {service} {{
                 rpc {method}({empty_message}) returns ({message});
               }}

               message {message} {{
                   int64 id = 1;
               }}"#
        );

        let file_descriptor = FileDescriptorProto {
            syntax: Some(syntax),
            package: Some(package),
            dependency: vec![annotations_import, empty_import, foo_import],
            message_type: vec![DescriptorProto {
                name: Some(message.clone()),
                field: vec![FieldDescriptorProto {
                    name: Some("id".to_string()),
                    number: Some(1),
                    r#type: Some(Type::Int64 as i32),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            service: vec![ServiceDescriptorProto {
                name: Some(service),
                method: vec![MethodDescriptorProto {
                    name: Some(method),
                    input_type: Some(empty_message),
                    output_type: Some(message),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };

        let result = super::parse(&input).unwrap();

        assert_eq!(file_descriptor, result)
    }
}
