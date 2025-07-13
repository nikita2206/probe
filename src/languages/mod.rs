pub mod rust;
pub mod python;
pub mod javascript;
pub mod go;
pub mod c;
pub mod java;
pub mod csharp;

#[cfg(test)]
mod tests;

// Re-export the language processors for easy access
pub use rust::RustProcessor;
pub use python::PythonProcessor;
pub use javascript::JavaScriptProcessor;
pub use go::GoProcessor;
pub use c::CProcessor;
pub use java::JavaProcessor;
pub use csharp::CSharpProcessor; 