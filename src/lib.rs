//! `trustrl` allows manipulating URLs.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod template;
pub mod transform;

pub use template::{RenderError, UrlTemplate};
pub use transform::{TransformError, UrlTransformation};
