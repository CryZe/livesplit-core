use livesplit_auto_splitting::{SettingValue, SettingsStore};

use crate::util::xml::{
    self,
    helper::{end_tag, Error as XmlError},
    Attributes, Event, Reader, TagName,
};

/// The Error type for splits files that couldn't be parsed by the LiveSplit
/// Parser.
#[derive(Debug, snafu::Snafu)]
pub enum Error {
    /// The underlying XML format couldn't be parsed.
    Xml {
        /// The underlying error.
        source: XmlError,
    },
    // /// Failed to parse an integer.
    // ParseInt {
    //     /// The underlying error.
    //     source: core::num::ParseIntError,
    // },
    // /// Failed to parse a floating point number.
    // ParseFloat {
    //     /// The underlying error.
    //     source: core::num::ParseFloatError,
    // },
    // /// Failed to parse a time.
    // ParseTime {
    //     /// The underlying error.
    //     source: crate::timing::ParseError,
    // },
    // /// Failed to parse a date.
    // ParseDate,
    // /// Parsed comparison has an invalid name.
    // InvalidComparisonName {
    //     /// The underlying error.
    //     source: ComparisonError,
    // },
    /// Failed to parse a boolean.
    ParseBool,
}

impl From<XmlError> for Error {
    fn from(source: XmlError) -> Self {
        Self::Xml { source }
    }
}

/// The Result type for the LiveSplit Parser.
pub type Result<T> = core::result::Result<T, Error>;

fn parse_bool(value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(Error::ParseBool),
    }
}

fn parse_tag_stream<F, E>(reader: &mut Reader<'_>, mut f: F) -> core::result::Result<(), E>
where
    F: FnMut(&mut Reader<'_>, TagName<'_>, Attributes<'_>) -> core::result::Result<(), E>,
    E: From<XmlError>,
{
    loop {
        match reader.read_event().ok_or(XmlError::Xml)? {
            Event::Start(start) => {
                let (name, attributes) = start.name_and_attributes();
                f(reader, name, attributes)?;
            }
            Event::End(_) => return Err(XmlError::UnexpectedElement).map_err(Into::into),
            Event::Ended => return Ok(()),
            _ => {}
        }
    }
}

pub fn parse(source: &str) -> Result<SettingsStore> {
    let mut reader = xml::Reader::new(source);

    let mut settings_store = SettingsStore::new();

    parse_tag_stream(&mut reader, |reader, tag, attributes| {
        if tag.name() == "Setting" {
            let mut key = None;
            let mut value = None;
            let mut value_type = "";

            for (k, v) in attributes.iter() {
                match k {
                    "key" => key = Some(v),
                    "value" => value = Some(v),
                    "type" => value_type = v.escaped(),
                    _ => {}
                }
            }

            let (Some(key), Some(value)) = (key, value) else {
            return Err(Error::Xml {
                source: XmlError::AttributeNotFound,
            });
        };

            let value = match value_type {
                "bool" => SettingValue::Bool(parse_bool(value.escaped())?),
                _ => {
                    // TODO: Better error
                    return Err(Error::Xml {
                        source: XmlError::AttributeNotFound,
                    });
                }
            };

            settings_store.set(key.unescape_str().into(), value);
        }

        end_tag(reader)
    })?;

    Ok(settings_store)
}
