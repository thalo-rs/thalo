use core::fmt;

use rdkafka::error::KafkaError;

#[derive(Debug)]
pub enum Error {
    Custom(CustomError),
    Kafka(KafkaError),
    TokioIo(tokio::io::Error),
}

#[derive(Debug)]
pub enum ErrorKind {
    Simple(String),
    NoStdout,
}

#[derive(Debug)]
pub struct CustomError {
    kind: ErrorKind,
    reason: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn new(reason: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        let reason = reason.into();
        Self::Custom(CustomError {
            kind: ErrorKind::Simple(reason.to_string()),
            reason: Some(reason),
        })
    }

    pub fn kafka(err: KafkaError) -> Self {
        Self::Kafka(err)
    }

    pub fn no_stdout() -> Self {
        Self::Custom(CustomError {
            kind: ErrorKind::NoStdout,
            reason: None,
        })
    }

    pub fn tokio_io(err: tokio::io::Error) -> Self {
        Self::TokioIo(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(custom) => match &custom.reason {
                Some(reason) => write!(f, "kind: {:?}, reason: {}", custom.kind, reason),
                None => write!(f, "kind: {:?}", custom.kind),
            },
            Self::Kafka(err) => write!(f, "kafka error: {}", err),
            Self::TokioIo(err) => write!(f, "tokio io error: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Custom(custom) => custom
                .reason
                .as_deref()
                .map(|err| err as &dyn std::error::Error),
            _ => None,
        }
    }
}

impl From<KafkaError> for Error {
    fn from(err: KafkaError) -> Self {
        Self::kafka(err)
    }
}

impl From<tokio::io::Error> for Error {
    fn from(err: tokio::io::Error) -> Self {
        Self::tokio_io(err)
    }
}
