use serde::{Deserialize, Serialize};

use crate::awto::{DomainCommand, DomainEvent, MajorMinorPatch};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    UserRegistered { username: String, password: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Command {
    RegisterUser { username: String, password: String },
}

impl DomainEvent for Event {
    fn event_type(&self) -> &'static str {
        match self {
            Self::UserRegistered { .. } => "user_registered",
        }
    }

    fn event_version(&self) -> MajorMinorPatch {
        match self {
            Self::UserRegistered { .. } => (0, 1, 0).into(),
        }
    }
}

impl DomainCommand for Command {
    fn command_type(&self) -> &'static str {
        match self {
            Self::RegisterUser { .. } => "register_user",
        }
    }

    fn command_version(&self) -> MajorMinorPatch {
        match self {
            Self::RegisterUser { .. } => (0, 1, 0).into(),
        }
    }
}
