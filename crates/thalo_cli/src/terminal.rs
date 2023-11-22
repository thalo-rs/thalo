//! Module for the terminal output implementation.
//!
//! This implementation is heavily inspired by `Shell` from `cargo`.
#![allow(dead_code)]

use std::cell::RefCell;
use std::fmt;
use std::io::{stderr, stdout, IsTerminal, Write};
use std::str::FromStr;

use anyhow::{bail, Result};
use owo_colors::{AnsiColors, OwoColorize};

/// The supported color options of `cargo`.
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Color {
    /// Automatically provide colorized output based on whether
    /// the output is a terminal.
    #[default]
    Auto,
    /// Never provide colorized output.
    Never,
    /// Always provide colorized output.
    Always,
}

impl FromStr for Color {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "never" => Ok(Self::Never),
            "always" => Ok(Self::Always),
            _ => bail!("argument for --color must be auto, always, or never, but found `{value}`"),
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Never => write!(f, "never"),
            Self::Always => write!(f, "always"),
        }
    }
}

/// The requested verbosity of output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verbosity {
    /// Verbose output.
    Verbose,
    /// Normal output.
    Normal,
    /// Quiet (no) output.
    Quiet,
}

pub(crate) struct TerminalState {
    pub(crate) output: Output,
    verbosity: Verbosity,
    pub(crate) needs_clear: bool,
}

impl TerminalState {
    /// Clears the current stderr line if needed.
    pub(crate) fn clear_stderr(&mut self) {
        if self.needs_clear {
            if self.output.supports_color() {
                imp::stderr_erase_line();
            }

            self.needs_clear = false;
        }
    }
}

impl fmt::Debug for TerminalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.output {
            Output::Write(_) => f
                .debug_struct("Terminal")
                .field("verbosity", &self.verbosity)
                .finish(),
            Output::Stream { color, .. } => f
                .debug_struct("Terminal")
                .field("verbosity", &self.verbosity)
                .field("color", color)
                .finish(),
        }
    }
}

/// An abstraction around output that considers preferences for verbosity and
/// color.
///
/// This is based off of the `cargo` implementation.
#[derive(Debug)]
pub struct Terminal(RefCell<TerminalState>);

impl Terminal {
    /// Creates a new terminal with the given verbosity and color.
    pub fn new(verbosity: Verbosity, color: Color) -> Self {
        Self(RefCell::new(TerminalState {
            output: Output::Stream {
                is_terminal: stderr().is_terminal(),
                color,
            },
            verbosity,
            needs_clear: false,
        }))
    }

    /// Creates a terminal from a plain writable object, with no color, and max
    /// verbosity.
    pub fn from_write(out: Box<dyn Write>) -> Self {
        Self(RefCell::new(TerminalState {
            output: Output::Write(out),
            verbosity: Verbosity::Verbose,
            needs_clear: false,
        }))
    }

    /// Prints a green 'status' message.
    pub fn status<T, U>(&self, status: T, message: U) -> Result<()>
    where
        T: fmt::Display,
        U: fmt::Display,
    {
        let status_green = status.green();

        let status = if self.0.borrow().output.supports_color() {
            &status_green as &dyn fmt::Display
        } else {
            &status
        };

        self.print(status, Some(&message), true)
    }

    /// Prints a 'status' message with the specified color.
    pub fn status_with_color<T, U>(
        &self,
        status: T,
        message: U,
        color: owo_colors::AnsiColors,
    ) -> Result<()>
    where
        T: fmt::Display,
        U: fmt::Display,
    {
        let status_color = status.color(color);

        let status = if self.0.borrow().output.supports_color() {
            &status_color as &dyn fmt::Display
        } else {
            &status
        };

        self.print(status, Some(&message), true)
    }

    /// Prints a cyan 'note' message.
    pub fn note<T: fmt::Display>(&self, message: T) -> Result<()> {
        let status = "note";
        let status_cyan = status.cyan();

        let status = if self.0.borrow().output.supports_color() {
            &status_cyan as &dyn fmt::Display
        } else {
            &status
        };

        self.print(status, Some(&message), false)
    }

    /// Prints a yellow 'warning' message.
    pub fn warn<T: fmt::Display>(&self, message: T) -> Result<()> {
        let status = "warning";
        let status_yellow = status.yellow();

        let status = if self.0.borrow().output.supports_color() {
            &status_yellow as &dyn fmt::Display
        } else {
            &status
        };

        self.print(status, Some(&message), false)
    }

    /// Prints a red 'error' message.
    pub fn error<T: fmt::Display>(&self, message: T) -> Result<()> {
        let status = "error";
        let status_red = status.red();

        let status = if self.0.borrow().output.supports_color() {
            &status_red as &dyn fmt::Display
        } else {
            &status
        };

        // This doesn't call print as errors are always printed even when quiet
        let mut state = self.0.borrow_mut();
        state.clear_stderr();
        state.output.print(status, Some(&message), false)
    }

    /// Write a styled fragment to stdout.
    ///
    /// Caller is responsible for deciding whether [`Shell::verbosity`] is
    /// affects output.
    pub fn write_stdout(
        &self,
        fragment: impl fmt::Display,
        color: Option<AnsiColors>,
    ) -> Result<()> {
        self.0.borrow_mut().output.write_stdout(fragment, color)
    }

    /// Prints a status that can be justified followed by a message.
    fn print(
        &self,
        status: &dyn fmt::Display,
        message: Option<&dyn fmt::Display>,
        justified: bool,
    ) -> Result<()> {
        let mut state = self.0.borrow_mut();
        match state.verbosity {
            Verbosity::Quiet => Ok(()),
            _ => {
                state.clear_stderr();
                state.output.print(status, message, justified)
            }
        }
    }

    /// Returns the width of the terminal in spaces, if any.
    pub fn width(&self) -> Option<usize> {
        match &self.0.borrow().output {
            Output::Stream { .. } => imp::stderr_width(),
            _ => None,
        }
    }

    /// Returns the verbosity of the terminal.
    pub fn verbosity(&self) -> Verbosity {
        self.0.borrow().verbosity
    }

    pub(crate) fn state_mut(&self) -> std::cell::RefMut<'_, TerminalState> {
        self.0.borrow_mut()
    }
}

/// A `Write`able object, either with or without color support.
pub(crate) enum Output {
    /// A plain write object without color support.
    Write(Box<dyn Write>),
    /// Color-enabled stdio, with information on whether color should be used
    Stream { is_terminal: bool, color: Color },
}

impl Output {
    pub(crate) fn supports_color(&self) -> bool {
        match self {
            Output::Write(_) => false,
            Output::Stream { is_terminal, color } => match color {
                Color::Auto => *is_terminal,
                Color::Never => false,
                Color::Always => true,
            },
        }
    }

    /// Prints out a message with a bold, optionally-justified status.
    pub(crate) fn print(
        &mut self,
        status: &dyn fmt::Display,
        message: Option<&dyn fmt::Display>,
        justified: bool,
    ) -> Result<()> {
        match *self {
            Output::Stream { .. } => {
                let stderr = &mut stderr();
                let status_bold = status.bold();

                let status = if self.supports_color() {
                    &status_bold as &dyn fmt::Display
                } else {
                    &status
                };

                if justified {
                    write!(stderr, "{status:>12}")?;
                } else {
                    write!(stderr, "{status}:")?;
                }

                match message {
                    Some(message) => writeln!(stderr, " {}", message)?,
                    None => write!(stderr, " ")?,
                }
            }
            Output::Write(ref mut w) => {
                if justified {
                    write!(w, "{status:>12}")?;
                } else {
                    write!(w, "{status}:")?;
                }
                match message {
                    Some(message) => writeln!(w, " {}", message)?,
                    None => write!(w, " ")?,
                }
            }
        }

        Ok(())
    }

    fn write_stdout(
        &mut self,
        fragment: impl fmt::Display,
        color: Option<AnsiColors>,
    ) -> Result<()> {
        match *self {
            Self::Stream { .. } => {
                let mut stdout = stdout();

                match color {
                    Some(color) => {
                        let colored_fragment = fragment.color(color);
                        let fragment: &dyn fmt::Display = if self.supports_color() {
                            &colored_fragment as &dyn fmt::Display
                        } else {
                            &fragment
                        };

                        write!(stdout, "{fragment}")?;
                    }
                    None => write!(stdout, "{fragment}")?,
                }
            }
            Self::Write(ref mut w) => {
                write!(w, "{fragment}")?;
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
mod imp {
    use std::mem;

    pub fn stderr_width() -> Option<usize> {
        unsafe {
            let mut winsize: libc::winsize = mem::zeroed();
            // The .into() here is needed for FreeBSD which defines TIOCGWINSZ
            // as c_uint but ioctl wants c_ulong.
            if libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ, &mut winsize) < 0 {
                return None;
            }
            if winsize.ws_col > 0 {
                Some(winsize.ws_col as usize)
            } else {
                None
            }
        }
    }

    pub fn stderr_erase_line() {
        // This is the "EL - Erase in Line" sequence. It clears from the cursor
        // to the end of line.
        // https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_sequences
        eprint!("\x1B[K");
    }
}

#[cfg(windows)]
mod imp {
    use std::{cmp, mem, ptr};

    use windows_sys::core::PCSTR;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileA, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        GetConsoleScreenBufferInfo, GetStdHandle, CONSOLE_SCREEN_BUFFER_INFO, STD_ERROR_HANDLE,
    };

    pub fn stderr_width() -> Option<usize> {
        unsafe {
            let stdout = GetStdHandle(STD_ERROR_HANDLE);
            let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = mem::zeroed();
            if GetConsoleScreenBufferInfo(stdout, &mut csbi) != 0 {
                return Some((csbi.srWindow.Right - csbi.srWindow.Left) as usize);
            }

            // On mintty/msys/cygwin based terminals, the above fails with
            // INVALID_HANDLE_VALUE. Use an alternate method which works
            // in that case as well.
            let h = CreateFileA(
                "CONOUT$\0".as_ptr() as PCSTR,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                0,
                0,
            );

            if h == INVALID_HANDLE_VALUE {
                return None;
            }

            let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = mem::zeroed();
            let rc = GetConsoleScreenBufferInfo(h, &mut csbi);
            CloseHandle(h);
            if rc != 0 {
                let width = (csbi.srWindow.Right - csbi.srWindow.Left) as usize;
                // Unfortunately cygwin/mintty does not set the size of the
                // backing console to match the actual window size. This
                // always reports a size of 80 or 120 (not sure what
                // determines that). Use a conservative max of 60 which should
                // work in most circumstances. ConEmu does some magic to
                // resize the console correctly, but there's no reasonable way
                // to detect which kind of terminal we are running in, or if
                // GetConsoleScreenBufferInfo returns accurate information.
                return Some(cmp::min(60, width));
            }

            None
        }
    }

    pub fn stderr_erase_line() {
        match stderr_width() {
            Some(width) => {
                let blank = " ".repeat(width);
                eprint!("{}\r", blank);
            }
            _ => (),
        }
    }
}
