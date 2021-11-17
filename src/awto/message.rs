use std::{collections::HashMap, error, fmt, str};

use super::DomainAggregate;

pub trait DomainEvent:
    Clone + std::fmt::Debug + PartialEq + serde::ser::Serialize + serde::de::DeserializeOwned
{
    /// A name specifying the event, used for event upcasting.
    fn event_type(&self) -> &'static str;

    /// A version of the `event_type`, use for event upcasting.
    fn event_version(&self) -> MajorMinorPatch;
}

pub trait DomainCommand:
    Clone + std::fmt::Debug + PartialEq + serde::ser::Serialize + serde::de::DeserializeOwned
{
    /// A name specifying the command, used for command upcasting.
    fn command_type(&self) -> &'static str;

    /// A version of the `command_type`, use for command upcasting.
    fn command_version(&self) -> MajorMinorPatch;
}

/// `EventEnvelope` is a data structure that encapsulates an event with along with it's pertinent
/// information. All of the associated data will be transported and persisted together.
///
/// Within any system an event must be unique based on its' `aggregate_type`, `aggregate_id` and
/// `sequence`.
#[derive(Debug)]
pub struct EventEnvelope<A>
where
    A: DomainAggregate,
{
    /// The id of the aggregate instance.
    pub aggregate_id: String,
    /// The sequence number for an aggregate instance.
    pub sequence: usize,
    /// The type of aggregate the event applies to.
    pub aggregate_type: String,
    /// The type of event.
    pub event_type: String,
    /// The event version.
    pub event_version: String,
    /// The event payload with all business information.
    pub payload: A::Event,
    /// Additional metadata for use in auditing, logging or debugging purposes.
    pub metadata: HashMap<String, String>,
}

impl<A> EventEnvelope<A>
where
    A: DomainAggregate,
{
    /// A convenience function for packaging an event in an `EventEnvelope`, used for
    /// testing `QueryProcessor`s.
    pub fn new(
        aggregate_id: String,
        sequence: usize,
        aggregate_type: String,
        payload: A::Event,
    ) -> Self {
        EventEnvelope {
            aggregate_id,
            sequence,
            aggregate_type,
            event_type: payload.event_type().to_string(),
            event_version: payload.event_version().to_string(),
            payload,
            metadata: Default::default(),
        }
    }
    /// A convenience function for packaging an event in an `EventEnvelope`, used for
    /// testing `QueryProcessor`s. This version allows custom metadata to also be processed.
    pub fn new_with_metadata(
        aggregate_id: String,
        sequence: usize,
        aggregate_type: String,
        payload: A::Event,
        metadata: HashMap<String, String>,
    ) -> Self {
        EventEnvelope {
            aggregate_id,
            sequence,
            aggregate_type,
            event_type: payload.event_type().to_string(),
            event_version: payload.event_version().to_string(),
            payload,
            metadata,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MajorMinorPatch {
    major: u16,
    minor: u16,
    patch: u16,
}

impl MajorMinorPatch {
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl From<(u16, u16, u16)> for MajorMinorPatch {
    fn from(v: (u16, u16, u16)) -> Self {
        Self {
            major: v.0,
            minor: v.1,
            patch: v.2,
        }
    }
}

#[derive(Debug)]
pub struct CorruptMajorMinorPatchInput;

impl fmt::Display for CorruptMajorMinorPatchInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "corrupt major.minor.patch input")
    }
}

impl error::Error for CorruptMajorMinorPatchInput {}

impl fmt::Display for MajorMinorPatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl str::FromStr for MajorMinorPatch {
    type Err = CorruptMajorMinorPatchInput;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('.');
        let major: u16 = split
            .next()
            .ok_or(CorruptMajorMinorPatchInput)?
            .parse()
            .map_err(|_| CorruptMajorMinorPatchInput)?;
        let minor: u16 = split
            .next()
            .ok_or(CorruptMajorMinorPatchInput)?
            .parse()
            .map_err(|_| CorruptMajorMinorPatchInput)?;
        let patch: u16 = split
            .next()
            .ok_or(CorruptMajorMinorPatchInput)?
            .parse()
            .map_err(|_| CorruptMajorMinorPatchInput)?;
        if split.next().is_some() {
            return Err(CorruptMajorMinorPatchInput);
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}
