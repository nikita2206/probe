pub mod java;

#[cfg(test)]
mod tests;

// Re-export the Java language processor for easy access
pub use java::JavaProcessor;
