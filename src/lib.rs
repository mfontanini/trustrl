//! `trustrl` allows manipulating URLs.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod parse;
pub mod template;
pub mod transform;

pub use parse::parse_url;
pub use template::{RenderError, UrlTemplate};
pub use transform::{TransformError, UrlTransformation};
