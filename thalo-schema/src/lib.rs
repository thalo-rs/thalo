pub use compiler::Compiler;
pub use error::Error;

mod compiler;
mod error;
pub mod schema;

pub fn configure() -> Compiler {
    Compiler::new()
}
