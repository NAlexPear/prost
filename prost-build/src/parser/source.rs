use nom::{character::complete::multispace0, multi::many0, IResult};
use nom_locate::LocatedSpan;
use prost_types::source_code_info::Location;
use std::cell::RefCell;

use super::comment;

pub(crate) const ROOT: () = ();

/// Give values the ability to define their own path generators
// FIXME: there's probably a pattern here that makes it possible to centralize indexing
// (i.e. unary/top-level paths, repeated paths, and append-to-parent paths)
pub(crate) trait Tag {
    /// Generate a path for this tag based on prior locations
    fn into_path(&self, locations: &[Location]) -> Vec<i32>;
}

/// "()" is the root path implementation
impl Tag for () {
    fn into_path(&self, _locations: &[Location]) -> Vec<i32> {
        Vec::new()
    }
}

/// Parsing state needed for any side effects within a parser
#[derive(Debug, Clone, Copy)]
pub(crate) struct State<'a>(&'a LocationRecorder);

impl<'a> State<'a> {
    /// Create a new [`State`] from a set of [`Location`]s
    pub(crate) fn new(location_recorder: &'a LocationRecorder) -> Self {
        Self(location_recorder)
    }

    /// start recording a location at a [`Span`], receiving a handle to that location for further updates
    /// FIXME: make this recording fallible with a custom (internal) error type
    fn record_location_start<T>(&self, start: Span<'a>, tag: T) -> LocationHandle
    where
        T: Tag,
    {
        let start_line = (start.location_line() - 1) as i32;
        let start_column = (start.get_column() - 1) as i32;

        // create a placeholder span with the maximum capacity
        let mut span = Vec::with_capacity(4);
        span.push(start_line);
        span.push(start_column);

        // generate a path from the tag
        let path = {
            let locations = self.0.locations.borrow();
            tag.into_path(&locations)
        };

        // create an in-progress location missing the span endings
        let location = Location {
            path,
            span,
            ..Default::default()
        };

        // inject the placeholder location into the set of recordings
        let mut locations = self.0.locations.borrow_mut();
        locations.push(location);

        LocationHandle {
            index: locations.len() - 1,
            leading_detached_comments: Vec::new(),
            leading_comments: None,
            trailing_comments: None,
        }
    }

    /// Consume a [`LocationHandle`] at a [`Span`]'s coordinates
    fn record_location_end(&self, handle: LocationHandle, end: Span<'a>) {
        let end_line = (end.location_line() - 1) as i32;
        let end_column = (end.get_column() - 1) as i32;

        if let Some(location) = &mut self.0.locations.borrow_mut().get_mut(handle.index) {
            // propagate the comments
            location.trailing_comments = handle.trailing_comments;
            location.leading_comments = handle.leading_comments.map(String::from);
            location.leading_detached_comments = handle
                .leading_detached_comments
                .into_iter()
                .map(String::from)
                .collect();

            // adjust the span with ending columns
            let span = &mut location.span;

            if span[0] != end_line {
                span.push(end_line);
            }

            span.push(end_column);
        }
    }

    /// Consume a [`LocationHandle`] and remove its children from the stack
    fn remove_location(&self, handle: LocationHandle) {
        self.0.locations.borrow_mut().drain(handle.index..);
    }

    #[cfg(test)]
    pub(crate) fn into_inner(&self) -> Vec<Location> {
        self.0.clone().into_inner()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LocationRecorder {
    locations: RefCell<Vec<Location>>,
}

impl LocationRecorder {
    /// Create a new, empty [`LocationRecorder`]
    pub(crate) fn new() -> Self {
        Self {
            locations: RefCell::new(Vec::new()),
        }
    }

    /// Consume the recorder, returning only valid locations
    pub(crate) fn into_inner(self) -> Vec<Location> {
        self.locations
            .into_inner()
            .into_iter()
            .filter(|location| location.span.len() > 2)
            .collect()
    }
}

/// Location-recording handle given out when `record_location_start` is called on [`State`]
pub(crate) struct LocationHandle<'a> {
    index: usize,
    leading_detached_comments: Vec<&'a str>,
    leading_comments: Option<&'a str>,
    trailing_comments: Option<String>,
}

/// Generic location-tracking input for use in parsers
pub(crate) type Span<'a> = LocatedSpan<&'a str, State<'a>>;

/// Wrapper function for handling location for a parser
pub(crate) fn locate<'a, T, F, O>(
    mut parser: F,
    tag: T,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, O>
where
    F: FnMut(Span<'a>) -> IResult<Span<'a>, O>,
    T: Tag + Copy,
{
    move |input| {
        // parse leading comments
        let (input, mut leading_detached_comments) = many0(comment::parse)(input)?;

        // consume any additional preceding whitespace
        let (start, leading_whitespace) = multispace0(input)?;

        // start recording the identifier's location
        let mut location_record = input.extra.record_location_start(start, tag);

        // check if the last leading comment was attached or not
        if leading_whitespace
            .lines()
            .filter(|line| line.is_empty())
            .count()
            == 1
        {
            let leading_comments = leading_detached_comments.pop();
            location_record.leading_comments = leading_comments;
        }

        // record the remaining comments as detached comments
        location_record.leading_detached_comments = leading_detached_comments;

        // run the wrapped function
        match parser(start) {
            Ok((end, output)) => {
                // finish recording the location
                input.extra.record_location_end(location_record, end);

                // consume remaining whitespace
                let (remainder, _) = multispace0(end)?;

                // pass on what's left
                Ok((remainder, output))
            }
            Err(error) => {
                input.extra.remove_location(location_record);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{LocationRecorder, Span, State, ROOT};

    #[test]
    fn handles_root_path() {
        let location_recorder = LocationRecorder::new();
        let state = State::new(&location_recorder);
        let span = Span::new_extra("", state);
        span.extra.record_location_start(span, ROOT);

        let expected = Vec::<i32>::new();
        let actual = &span.extra.0.locations.borrow()[0].path;
        assert_eq!(&expected, actual)
    }
}
