//! Provides the Text Component and relevant types for using it. The Text
//! Component simply visualizes any given text. This can either be a single
//! centered text, or split up into a left and right text, which is suitable for
//! a situation where you have a label and a value.

use super::DEFAULT_KEY_VALUE_GRADIENT;
use crate::settings::{Color, Field, Gradient, SettingsDescription, Value};
use serde::{Deserialize, Serialize};
use serde_json::{to_writer, Result};
use std::borrow::Cow;
use std::io::Write;
use std::mem::replace;

/// The Text Component simply visualizes any given text. This can either be a
/// single centered text, or split up into a left and right text, which is
/// suitable for a situation where you have a label and a value.
#[derive(Default, Clone)]
pub struct Component {
    settings: Settings,
}

/// The Settings for this component.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The background shown behind the component.
    pub background: Gradient,
    /// Specifies whether to display the left and right text is supposed to be
    /// displayed as two rows.
    pub display_two_rows: bool,
    /// The color of the left part of the split up text or the whole text if
    /// it's not split up. If `None` is specified, the color is taken from the
    /// layout.
    pub left_center_color: Option<Color>,
    /// The color of the right part of the split up text. This can be ignored if
    /// the text is not split up. If `None` is specified, the color is taken
    /// from the layout.
    pub right_color: Option<Color>,
    /// The text to be shown.
    pub text: Text,
}

/// The text that is supposed to be shown.
#[derive(Clone, Serialize, Deserialize)]
pub enum Text {
    /// A single centered text.
    Center(String),
    /// A text that is split up into a left and right part. This is suitable for
    /// a situation where you have a label and a value.
    Split(String, String),
}

impl Text {
    /// Returns whether the text is split up into a left and right part.
    pub fn is_split(&self) -> bool {
        match self {
            Text::Split(_, _) => true,
            Text::Center(_) => false,
        }
    }

    /// Sets the centered text. If the current mode is split, it is switched to
    /// centered mode.
    pub fn set_center<S: Into<String>>(&mut self, text: S) {
        let text = text.into();
        if let Text::Center(inner) = self {
            *inner = text;
        } else {
            *self = Text::Center(text);
        }
    }

    /// Sets the left text. If the current mode is centered, it is switched to
    /// split mode, with the right text being empty.
    pub fn set_left<S: Into<String>>(&mut self, text: S) {
        let text = text.into();
        if let Text::Split(inner, _) = self {
            *inner = text;
        } else {
            *self = Text::Split(text, String::from(""));
        }
    }

    /// Sets the right text. If the current mode is centered, it is switched to
    /// split mode, with the left text being empty.
    pub fn set_right<S: Into<String>>(&mut self, text: S) {
        let text = text.into();
        if let Text::Split(_, inner) = self {
            *inner = text;
        } else {
            *self = Text::Split(String::from(""), text);
        }
    }
}

/// The state object describes the information to visualize for this component.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// The background shown behind the component.
    pub background: Gradient,
    /// Specifies whether to display the left and right text is supposed to be
    /// displayed as two rows.
    pub display_two_rows: bool,
    /// The color of the left part of the split up text or the whole text if
    /// it's not split up. If `None` is specified, the color is taken from the
    /// layout.
    pub left_center_color: Option<Color>,
    /// The color of the right part of the split up text. This can be ignored if
    /// the text is not split up. If `None` is specified, the color is taken
    /// from the layout.
    pub right_color: Option<Color>,
    /// The text to show for the component.
    pub text: Text,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            background: DEFAULT_KEY_VALUE_GRADIENT,
            display_two_rows: false,
            left_center_color: None,
            right_color: None,
            text: Text::Center(String::from("")),
        }
    }
}

impl State {
    /// Encodes the state object's information as JSON.
    pub fn write_json<W>(&self, writer: W) -> Result<()>
    where
        W: Write,
    {
        to_writer(writer, self)
    }
}

impl Component {
    /// Creates a new Text Component.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new Text Component with the given settings.
    pub fn with_settings(settings: Settings) -> Self {
        Self { settings }
    }

    /// Accesses the settings of the component.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Grants mutable access to the settings of the component.
    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    /// Accesses the name of the component.
    pub fn name(&self) -> Cow<'_, str> {
        let name: Cow<'_, str> = match &self.settings.text {
            Text::Center(text) => text.as_str().into(),
            Text::Split(left, right) => {
                let mut name = String::with_capacity(left.len() + right.len() + 1);
                name.push_str(left);
                if !left.is_empty() && !right.is_empty() {
                    name.push_str(" ");
                }
                name.push_str(right);
                name.into()
            }
        };

        if name.trim().is_empty() {
            "Text".into()
        } else {
            name
        }
    }

    /// Calculates the component's state.
    pub fn state(&self) -> State {
        State {
            background: self.settings.background,
            display_two_rows: self.settings.text.is_split() && self.settings.display_two_rows,
            left_center_color: self.settings.left_center_color,
            right_color: self.settings.right_color,
            text: self.settings.text.clone(),
        }
    }

    /// Accesses a generic description of the settings available for this
    /// component and their current values.
    pub fn settings_description(&self) -> SettingsDescription {
        let (first, second, color_name) = match &self.settings.text {
            Text::Center(text) => (
                Field::new("Text".into(), text.to_string().into()),
                None,
                "Text Color",
            ),
            Text::Split(left, right) => (
                Field::new("Left".into(), left.to_string().into()),
                Some(Field::new("Right".into(), right.to_string().into())),
                "Left Color",
            ),
        };

        let mut fields = vec![
            Field::new("Background".into(), self.settings.background.into()),
            Field::new("Split".into(), second.is_some().into()),
            first,
            Field::new(color_name.into(), self.settings.left_center_color.into()),
        ];

        if let Some(second) = second {
            fields.push(second);
            fields.push(Field::new(
                "Right Color".into(),
                self.settings.right_color.into(),
            ));
            fields.push(Field::new(
                "Display 2 Rows".into(),
                self.settings.display_two_rows.into(),
            ));
        }

        SettingsDescription::with_fields(fields)
    }

    /// Sets a setting's value by its index to the given value.
    ///
    /// # Panics
    ///
    /// This panics if the type of the value to be set is not compatible with
    /// the type of the setting's value. A panic can also occur if the index of
    /// the setting provided is out of bounds.
    pub fn set_value(&mut self, index: usize, value: Value) {
        match index {
            0 => self.settings.background = value.into(),
            1 => {
                self.settings.text = match (value.into_bool().unwrap(), &mut self.settings.text) {
                    (true, Text::Center(center)) => {
                        self.settings.right_color = self.settings.left_center_color;
                        self.settings.display_two_rows = false;

                        Text::Split(replace(center, String::new()), String::new())
                    }
                    (false, Text::Split(left, right)) => {
                        let mut value = replace(left, String::new());
                        let right = replace(right, String::new());
                        if !value.is_empty() && !right.is_empty() {
                            value.push(' ');
                        }
                        value.push_str(&right);

                        Text::Center(value)
                    }
                    _ => return,
                };
            }
            2 => match &mut self.settings.text {
                Text::Center(center) => *center = value.into(),
                Text::Split(left, _) => *left = value.into(),
            },
            3 => self.settings.left_center_color = value.into(),
            4 => match &mut self.settings.text {
                Text::Center(_) => panic!("Set right text when there's only a center text"),
                Text::Split(_, right) => *right = value.into(),
            },
            5 => self.settings.right_color = value.into(),
            6 => self.settings.display_two_rows = value.into(),
            _ => panic!("Unsupported Setting Index"),
        }
    }
}
