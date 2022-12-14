use super::{
    import::{self, Import},
    message,
    package::{self, Package},
    r#enum, service,
    source::{self, locate, ROOT},
    syntax,
};
use nom::{
    branch::alt,
    combinator::{iterator, map},
    IResult,
};
use prost_types::{
    DescriptorProto, EnumDescriptorProto, FileDescriptorProto, ServiceDescriptorProto,
};
use source::Span;

/// Allowed top-level statements within a file
enum Statement<'a> {
    Import(Import<'a>),
    Package(Package<'a>),
    Message(DescriptorProto),
    Service(ServiceDescriptorProto),
    Enum(EnumDescriptorProto),
    // FIXME: handle all the rest of the allowed statements
}

/// Parse a file and all of its child statements
pub(crate) fn parse<'a>(input: Span<'a>) -> IResult<Span<'a>, FileDescriptorProto> {
    locate(
        |input| {
            // consume the required syntax statement at the top of the file
            let (input, syntax) = syntax::parse(input)?;

            // create the placeholder protobuf
            let mut file_descriptor = FileDescriptorProto {
                syntax: Some(syntax.to_string()),
                ..Default::default()
            };

            // consume top-level statements until the file is finished
            let mut statements = iterator(
                input,
                alt((
                    map(import::parse, Statement::Import),
                    map(package::parse, Statement::Package),
                    map(message::parse, Statement::Message),
                    map(service::parse, Statement::Service),
                    map(r#enum::parse, Statement::Enum),
                )),
            );

            for statement in &mut statements {
                match statement {
                    Statement::Package(package) => {
                        if file_descriptor.package.is_some() {
                            // FIXME: return a "duplicate package" error
                        }

                        file_descriptor.package = Some(package.to_string());
                    }
                    Statement::Import(import) => {
                        file_descriptor.dependency.push(import.to_string())
                    }
                    Statement::Message(message) => file_descriptor.message_type.push(message),
                    Statement::Service(service) => file_descriptor.service.push(service),
                    Statement::Enum(r#enum) => file_descriptor.enum_type.push(r#enum),
                }
            }

            let (end, _) = statements.finish()?;

            Ok((end, file_descriptor))
        },
        ROOT,
    )(input)
}
