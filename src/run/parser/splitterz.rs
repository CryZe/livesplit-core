//! Provides the parser for SplitterZ splits files.

use crate::{settings::Image, timing, RealTime, Run, Segment, TimeSpan};
use snafu::ResultExt;
use std::borrow::Cow;
use std::io::{self, BufRead};
use std::num::ParseIntError;
use std::result::Result as StdResult;

/// The Error type for splits files that couldn't be parsed by the SplitterZ
/// Parser.
#[derive(Debug, snafu::Snafu)]
pub enum Error {
    /// An empty splits file was provided.
    Empty,
    /// Expected the name of the category, but didn't find it.
    ExpectedCategoryName,
    /// Expected the attempt count, but didn't find it.
    ExpectedAttemptCount,
    /// Failed to parse the amount of attempts.
    ParseAttemptCount {
        /// The underlying error.
        source: ParseIntError,
    },
    /// Expected the name of the segment, but didn't find it.
    ExpectedSegmentName,
    /// Expected the split time, but didn't find it.
    ExpectedSplitTime,
    /// Failed to parse a split time.
    ParseSplitTime {
        /// The underlying error.
        source: timing::ParseError,
    },
    /// Expected the best segment time, but didn't find it.
    ExpectedBestSegment,
    /// Failed to parse a best segment time.
    ParseBestSegment {
        /// The underlying error.
        source: timing::ParseError,
    },
    /// Failed to read the title line.
    TitleLine {
        /// The underlying error.
        source: io::Error,
    },
    /// Failed to read the next line.
    Line {
        /// The underlying error.
        source: io::Error,
    },
}

/// The Result type for the SplitterZ parser.
pub type Result<T> = StdResult<T, Error>;

fn unescape(text: &str) -> Cow<'_, str> {
    if text.contains('‡') {
        text.replace('‡', ",").into()
    } else {
        text.into()
    }
}

/// Attempts to parse a SplitterZ splits file. In addition to the source to
/// parse, you need to specify if additional files for the icons should be
/// loaded from the file system. If you are using livesplit-core in a
/// server-like environment, set this to `false`. Only client-side applications
/// should set this to `true`.
pub fn parse<R: BufRead>(source: R, load_icons: bool) -> Result<Run> {
    let mut run = Run::new();

    let mut icon_buf = Vec::new();

    let mut lines = source.lines();
    let line = lines.next().ok_or(Error::Empty)?.context(TitleLine)?;
    let mut splits = line.split(',');
    // Title Stuff here, do later
    run.set_category_name(unescape(splits.next().ok_or(Error::ExpectedCategoryName)?));
    run.set_attempt_count(
        splits
            .next()
            .ok_or(Error::ExpectedAttemptCount)?
            .parse()
            .context(ParseAttemptCount)?,
    );

    for line in lines {
        let line = line.context(Line)?;
        if !line.is_empty() {
            let mut splits = line.split(',');

            let mut segment =
                Segment::new(unescape(splits.next().ok_or(Error::ExpectedSegmentName)?));

            let time: TimeSpan = splits
                .next()
                .ok_or(Error::ExpectedSplitTime)?
                .parse()
                .context(ParseSplitTime)?;
            if time != TimeSpan::zero() {
                segment.set_personal_best_split_time(RealTime(Some(time)).into());
            }

            let time: TimeSpan = splits
                .next()
                .ok_or(Error::ExpectedBestSegment)?
                .parse()
                .context(ParseBestSegment)?;
            if time != TimeSpan::zero() {
                segment.set_best_segment_time(RealTime(Some(time)).into());
            }

            if load_icons {
                if let Some(icon_path) = splits.next() {
                    if !icon_path.is_empty() {
                        if let Ok(image) =
                            Image::from_file(unescape(icon_path).as_ref(), &mut icon_buf)
                        {
                            segment.set_icon(image);
                        }
                    }
                }
            }

            run.push_segment(segment);
        } else {
            break;
        }
    }

    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters() {
        const RUN: &[u8] = br#"Run Title:,1
SegmentName,0:00:00.00,0.00
SegmentName,0:00:00.00,0.00
SegmentName,0:00:00.00,0.00

Counter,1,True
Counter,1,True
"#;

        let run = parse(RUN, false).unwrap();
        assert_eq!(run.len(), 3);
    }
}
