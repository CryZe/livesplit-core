use super::{
    current_comparison, current_pace, delta, key_value, pb_chance, possible_time_save,
    previous_segment, segment_time, sum_of_best, total_playtime,
};
use crate::platform::prelude::*;
use crate::settings::{SettingsDescription, Value};
use crate::{AtomicDateTime, GeneralLayoutSettings, TimeSpan, Timer};
use alloc::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
enum InnerComponent {
    CurrentComparison(current_comparison::Component),
    CurrentPace(current_pace::Component),
    Delta(delta::Component),
    PbChance(pb_chance::Component),
    PossibleTimeSave(possible_time_save::Component),
    PreviousSegment(previous_segment::Component),
    SegmentTime(segment_time::Component),
    SumOfBest(sum_of_best::Component),
    TotalPlaytime(total_playtime::Component),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum InnerComponentSettings {
    CurrentComparison(current_comparison::Settings),
    CurrentPace(current_pace::Settings),
    Delta(delta::Settings),
    PbChance(pb_chance::Settings),
    PossibleTimeSave(possible_time_save::Settings),
    PreviousSegment(previous_segment::Settings),
    SegmentTime(segment_time::Settings),
    SumOfBest(sum_of_best::Settings),
    TotalPlaytime(total_playtime::Settings),
}

// TODO: Disallow empty component list.

#[derive(Clone)]
pub struct Component {
    components: Vec<InnerComponent>,
    queue: VecDeque<usize>,
    current_index: usize,
    last_present_time: Vec<AtomicDateTime>,
    values: Vec<Box<str>>,
}

impl Default for Component {
    fn default() -> Self {
        Self::with_settings(Default::default())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub components: Vec<InnerComponentSettings>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            components: vec![
                InnerComponentSettings::PreviousSegment(Default::default()),
                InnerComponentSettings::PossibleTimeSave(Default::default()),
                InnerComponentSettings::SumOfBest(Default::default()),
            ],
        }
    }
}

impl Component {
    /// Creates a new Current Comparison Component.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new Current Comparison Component with the given settings.
    pub fn with_settings(settings: Settings) -> Self {
        let components = settings
            .components
            .into_iter()
            .map(|s| match s {
                InnerComponentSettings::CurrentComparison(s) => InnerComponent::CurrentComparison(
                    current_comparison::Component::with_settings(s),
                ),
                InnerComponentSettings::CurrentPace(s) => {
                    InnerComponent::CurrentPace(current_pace::Component::with_settings(s))
                }
                InnerComponentSettings::Delta(s) => {
                    InnerComponent::Delta(delta::Component::with_settings(s))
                }
                InnerComponentSettings::PbChance(s) => {
                    InnerComponent::PbChance(pb_chance::Component::with_settings(s))
                }
                InnerComponentSettings::PossibleTimeSave(s) => InnerComponent::PossibleTimeSave(
                    possible_time_save::Component::with_settings(s),
                ),
                InnerComponentSettings::PreviousSegment(s) => {
                    InnerComponent::PreviousSegment(previous_segment::Component::with_settings(s))
                }
                InnerComponentSettings::SegmentTime(s) => {
                    InnerComponent::SegmentTime(segment_time::Component::with_settings(s))
                }
                InnerComponentSettings::SumOfBest(s) => {
                    InnerComponent::SumOfBest(sum_of_best::Component::with_settings(s))
                }
                InnerComponentSettings::TotalPlaytime(s) => {
                    InnerComponent::TotalPlaytime(total_playtime::Component::with_settings(s))
                }
            })
            .collect::<Vec<_>>();

        Self {
            last_present_time: vec![AtomicDateTime::now(); components.len()],
            values: vec!["".into(); components.len()],
            current_index: 0,
            queue: (1..components.len()).collect(),
            components,
        }
    }

    /// Accesses the settings of the component.
    pub fn settings(&self) -> Settings {
        let components = self
            .components
            .iter()
            .map(|c| match c {
                InnerComponent::CurrentComparison(c) => {
                    InnerComponentSettings::CurrentComparison(c.settings().clone())
                }
                InnerComponent::CurrentPace(c) => {
                    InnerComponentSettings::CurrentPace(c.settings().clone())
                }
                InnerComponent::Delta(c) => InnerComponentSettings::Delta(c.settings().clone()),
                InnerComponent::PbChance(c) => {
                    InnerComponentSettings::PbChance(c.settings().clone())
                }
                InnerComponent::PossibleTimeSave(c) => {
                    InnerComponentSettings::PossibleTimeSave(c.settings().clone())
                }
                InnerComponent::PreviousSegment(c) => {
                    InnerComponentSettings::PreviousSegment(c.settings().clone())
                }
                InnerComponent::SegmentTime(c) => {
                    InnerComponentSettings::SegmentTime(c.settings().clone())
                }
                InnerComponent::SumOfBest(c) => {
                    InnerComponentSettings::SumOfBest(c.settings().clone())
                }
                InnerComponent::TotalPlaytime(c) => {
                    InnerComponentSettings::TotalPlaytime(c.settings().clone())
                }
            })
            .collect();
        Settings { components }
    }

    /// Accesses the name of the component.
    pub fn name(&self) -> &'static str {
        "Slideshow"
    }

    /// Calculates the component's state based on the timer provided.
    pub fn state(
        &mut self,
        timer: &Timer,
        layout_settings: &GeneralLayoutSettings,
    ) -> key_value::State {
        let auto_requeue_time = TimeSpan::from_seconds(self.components.len() as f64 * 10.0);

        let now = AtomicDateTime::now();

        if (now - self.last_present_time[self.current_index]) > TimeSpan::from_seconds(5.0) {
            if let Some(new_index) = self.queue.pop_front() {
                self.current_index = new_index;
                self.last_present_time[self.current_index] = now;
            }
        }

        for (i, (component, old_value)) in self.components.iter().zip(&mut self.values).enumerate()
        {
            if i == self.current_index {
                continue;
            }

            let state = match component {
                InnerComponent::CurrentComparison(c) => c.state(timer),
                InnerComponent::CurrentPace(c) => c.state(timer),
                InnerComponent::Delta(c) => c.state(timer, layout_settings),
                InnerComponent::PbChance(c) => c.state(timer),
                InnerComponent::PossibleTimeSave(c) => c.state(timer),
                InnerComponent::PreviousSegment(c) => c.state(timer, layout_settings),
                InnerComponent::SegmentTime(c) => c.state(timer),
                InnerComponent::SumOfBest(c) => c.state(timer),
                InnerComponent::TotalPlaytime(c) => c.state(timer),
            };

            if *old_value != state.value && !self.queue.contains(&i) {
                *old_value = state.value;
                self.queue.push_back(i);
            }
        }

        for (i, last_present_time) in self.last_present_time.iter().enumerate() {
            if (now - *last_present_time) > auto_requeue_time && !self.queue.contains(&i) {
                self.queue.push_back(i);
            }
        }

        let state = match &self.components[self.current_index] {
            InnerComponent::CurrentComparison(c) => c.state(timer),
            InnerComponent::CurrentPace(c) => c.state(timer),
            InnerComponent::Delta(c) => c.state(timer, layout_settings),
            InnerComponent::PbChance(c) => c.state(timer),
            InnerComponent::PossibleTimeSave(c) => c.state(timer),
            InnerComponent::PreviousSegment(c) => c.state(timer, layout_settings),
            InnerComponent::SegmentTime(c) => c.state(timer),
            InnerComponent::SumOfBest(c) => c.state(timer),
            InnerComponent::TotalPlaytime(c) => c.state(timer),
        };
        self.values[self.current_index] = state.value.clone();
        state
    }

    /// Accesses a generic description of the settings available for this
    /// component and their current values.
    pub fn settings_description(&self) -> SettingsDescription {
        SettingsDescription::with_fields(vec![])
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
            _ => panic!("Unsupported Setting Index"),
        }
    }
}
