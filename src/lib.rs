//! `trustrl` allows manipulating URLs.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod transform;

pub use transform::{TransformError, UrlTransformation};
