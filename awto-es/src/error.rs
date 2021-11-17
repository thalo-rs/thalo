use std::{error, fmt};

pub struct Error {
    repr: Repr,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.repr, f)
    }
}

enum Repr {
    Simple(ErrorKind),
    // &str is a fat pointer, but &&str is a thin pointer.
    // SimpleMessage(ErrorKind, &'static &'static str),
    Custom(Box<Custom>),
}

#[derive(Debug)]
struct Custom {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ErrorKind {
    AlreadySeenEvent,
    DeserializeError,
    InternalError,
    MailboxFull,
    MissingKey,
    Other,
    ResourceNotFound,
    SerializeError,
    ValidationError,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        use ErrorKind::*;
        match *self {
            AlreadySeenEvent => "already seen event",
            DeserializeError => "deserialize error",
            InternalError => "internal error",
            MailboxFull => "mailbox full",
            MissingKey => "missing key",
            Other => "other error",
            ResourceNotFound => "resource not found",
            SerializeError => "serialize error",
            ValidationError => "validation error",
        }
    }
}

impl From<ErrorKind> for Error {
    /// Converts an [`ErrorKind`] into an [`Error`].
    ///
    /// This conversion allocates a new error with a simple representation of error kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Error, ErrorKind};
    ///
    /// let not_found = ErrorKind::NotFound;
    /// let error = Error::from(not_found);
    /// assert_eq!("entity not found", format!("{}", error));
    /// ```
    #[inline]
    fn from(kind: ErrorKind) -> Error {
        Error {
            repr: Repr::Simple(kind),
        }
    }
}

impl Error {
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            repr: Repr::Custom(Box::new(Custom {
                kind,
                error: error.into(),
            })),
        }
    }

    pub fn new_simple(kind: ErrorKind) -> Error {
        Error {
            repr: Repr::Simple(kind),
        }
    }

    pub fn validation_error<E>(msg: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error::new(ErrorKind::ValidationError, msg)
    }

    #[inline]
    pub fn get_ref(&self) -> Option<&(dyn error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::Simple(..) => None,
            // Repr::SimpleMessage(..) => None,
            Repr::Custom(ref c) => Some(&*c.error),
        }
    }

    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut (dyn error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::Simple(..) => None,
            // Repr::SimpleMessage(..) => None,
            Repr::Custom(ref mut c) => Some(&mut *c.error),
        }
    }

    #[inline]
    pub fn into_inner(self) -> Option<Box<dyn error::Error + Send + Sync>> {
        match self.repr {
            Repr::Simple(..) => None,
            // Repr::SimpleMessage(..) => None,
            Repr::Custom(c) => Some(c.error),
        }
    }

    #[inline]
    pub fn kind(&self) -> ErrorKind {
        match self.repr {
            Repr::Custom(ref c) => c.kind,
            Repr::Simple(kind) => kind,
            // Repr::SimpleMessage(kind, _) => kind,
        }
    }
}

impl fmt::Debug for Repr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Repr::Custom(ref c) => fmt::Debug::fmt(&c, fmt),
            Repr::Simple(kind) => fmt.debug_tuple("Kind").field(&kind).finish(),
            // Repr::SimpleMessage(kind, &message) => fmt
            //     .debug_struct("Error")
            //     .field("kind", &kind)
            //     .field("message", &message)
            //     .finish(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.repr {
            Repr::Custom(ref c) => c.error.fmt(fmt),
            Repr::Simple(kind) => write!(fmt, "{}", kind.as_str()),
            // Repr::SimpleMessage(_, &msg) => msg.fmt(fmt),
        }
    }
}

impl error::Error for Error {
    #[allow(deprecated, deprecated_in_future)]
    fn description(&self) -> &str {
        match self.repr {
            Repr::Simple(..) => self.kind().as_str(),
            // Repr::SimpleMessage(_, &msg) => msg,
            Repr::Custom(ref c) => c.error.description(),
        }
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn error::Error> {
        match self.repr {
            Repr::Simple(..) => None,
            // Repr::SimpleMessage(..) => None,
            Repr::Custom(ref c) => c.error.cause(),
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.repr {
            Repr::Simple(..) => None,
            // Repr::SimpleMessage(..) => None,
            Repr::Custom(ref c) => c.error.source(),
        }
    }
}

pub trait InternalError<T> {
    /// Maps a result to a `Result<T, Error>` where Error is an `ErrorKind::InternalError`.
    fn internal_error(self, msg: &str) -> Result<T, Error>;
}

impl<T, E> InternalError<T> for Result<T, E>
where
    E: fmt::Display,
{
    fn internal_error(self, msg: &str) -> Result<T, Error> {
        self.map_err(|err| {
            Error::new(
                ErrorKind::InternalError,
                format!("{}: {}", msg, err.to_string()),
            )
        })
    }
}
