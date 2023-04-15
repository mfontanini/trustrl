//! `trustrl` allows manipulating URLs.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod parse;
pub mod render;
pub mod transform;

pub use parse::parse_url;
pub use render::{RenderError, UrlRenderer, UrlTemplate};
pub use transform::{TransformError, UrlTransformation};
pub use url::Url;
