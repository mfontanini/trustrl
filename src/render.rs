//! URL rendering.

use runtime_format::{FormatArgs, FormatKey, FormatKeyError};
use serde::Serialize;
use std::{borrow::Cow, io::Write};
use url::Url;

/// Allows rendering URLs.
pub enum UrlRenderer<'a> {
    /// A renderer based on a template.
    Template(UrlTemplate<'a>),

    /// A JSON-based renderer.
    Json,
}

impl<'a> UrlRenderer<'a> {
    /// Constructs a new templated renderer.
    pub fn templated(format: &'a str) -> Self {
        Self::Template(UrlTemplate::new(format))
    }

    /// Construct a JSON-based renderer.
    pub fn json() -> Self {
        Self::Json
    }

    /// Render a URL into the given writer.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::io::BufWriter;
    /// # use url::Url;
    /// # use trustrl::UrlRenderer;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let url = Url::parse("https://example.com/foo")?;
    /// let renderer = UrlRenderer::templated("scheme is {scheme}, path is {path}");
    ///
    /// let mut writer = BufWriter::new(Vec::new());
    /// renderer.render(&url, &mut writer)?;
    /// assert_eq!(writer.into_inner()?, b"scheme is https, path is /foo");
    /// # Ok(())
    /// # }
    /// ````
    pub fn render<W: Write>(&self, url: &Url, writer: &mut W) -> Result<(), RenderError> {
        use UrlRenderer::*;
        match self {
            Template(template) => template.render(url, writer),
            Json => Self::render_json(url, writer),
        }
    }

    fn render_json<W: Write>(url: &Url, writer: &mut W) -> Result<(), RenderError> {
        serde_json::to_writer(writer, &JsonUrl::from(url))?;
        Ok(())
    }
}

/// A URL template.
///
/// This can be used to render a URL using a format string. Format strings should use a syntax like
/// `"render {something}"` where `something` will be replaced with whatever the rendered URL contains
/// under that key.
///
/// Valid keys in the format string are:
/// * url
/// * scheme
/// * host
/// * port
/// * user
/// * password
/// * path
/// * query
/// * fragment
pub struct UrlTemplate<'a> {
    format: &'a str,
}

impl<'a> UrlTemplate<'a> {
    /// Construct a new URL template.
    pub fn new(format: &'a str) -> Self {
        Self { format }
    }

    /// Use this template to render a URL.
    pub fn render<W: Write>(&self, url: &Url, writer: &mut W) -> Result<(), RenderError> {
        let formatter = UrlFormatter { url };
        let args = FormatArgs::new(self.format, &formatter);
        write!(writer, "{args}")?;
        Ok(())
    }
}

/// An error during the rendering of a URL.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// An IO error during the formatting.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// JSON serialization failed.
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

struct UrlFormatter<'a> {
    url: &'a Url,
}

impl<'a> FormatKey for UrlFormatter<'a> {
    fn fmt(&self, key: &str, f: &mut core::fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
        if key == "port" {
            let output = match PortFormatter::new(self.url).port() {
                Some(port) => write!(f, "{port}"),
                None => write!(f, ""),
            };
            return output.map_err(FormatKeyError::Fmt);
        }
        let value = match key {
            "url" => self.url.as_str(),
            "scheme" => self.url.scheme(),
            "host" => self.url.host_str().unwrap_or(""),
            "user" => self.url.username(),
            "password" => self.url.password().unwrap_or(""),
            "path" => self.url.path(),
            "query" => self.url.query().unwrap_or(""),
            "fragment" => self.url.fragment().unwrap_or(""),
            _ => return Err(FormatKeyError::UnknownKey),
        };
        write!(f, "{value}").map_err(FormatKeyError::Fmt)
    }
}

struct PortFormatter<'a> {
    url: &'a Url,
}

impl<'a> PortFormatter<'a> {
    fn new(url: &'a Url) -> Self {
        Self { url }
    }

    fn port(&self) -> Option<u16> {
        if let Some(port) = self.url.port() {
            Some(port)
        } else {
            Self::scheme_port(self.url.scheme())
        }
    }

    fn scheme_port(scheme: &str) -> Option<u16> {
        match scheme {
            "http" | "ws" | "rtmpt" | "rtmpte" => Some(80),
            "https" | "wss" | "rtmps" | "rtmpts" => Some(443),
            "ftp" => Some(21),
            "ftps" => Some(990),
            "scp" | "ssh" | "sftp" => Some(22),
            "smtp" => Some(25),
            "smtps" => Some(465),
            "telnet" => Some(21),
            "ldap" => Some(389),
            "ldaps" => Some(636),
            "pop3" => Some(110),
            "pop3s" => Some(995),
            "smb" | "smbs" => Some(445),
            "rtsp" => Some(554),
            "mqtt" => Some(1883),
            "gopher" => Some(70),
            "rtmp" | "rtmpe" => Some(1935),
            _ => None,
        }
    }
}

#[derive(Serialize)]
struct JsonUrl<'a> {
    url: &'a str,
    scheme: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    path: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<&'a str>,
    params: Vec<JsonQueryParam<'a>>,
}

#[derive(Serialize)]
struct JsonQueryParam<'a> {
    key: Cow<'a, str>,
    value: Cow<'a, str>,
}

impl<'a> From<&'a Url> for JsonUrl<'a> {
    fn from(url: &'a Url) -> Self {
        let params: Vec<_> = url.query_pairs().map(|(key, value)| JsonQueryParam { key, value }).collect();
        let user = if url.username().is_empty() { None } else { Some(url.username()) };
        JsonUrl {
            url: url.as_str(),
            user,
            password: url.password(),
            scheme: url.scheme(),
            host: url.host_str(),
            port: PortFormatter::new(url).port(),
            path: url.path(),
            query: url.query(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::io::BufWriter;

    fn render_to_string(renderer: UrlRenderer, url: &Url) -> Result<String, ()> {
        let mut writer = BufWriter::new(Vec::new());
        renderer.render(url, &mut writer).map_err(|_| ())?;

        let buffer = writer.into_inner().map_err(|_| ())?;
        let formatted = String::from_utf8(buffer).map_err(|_| ())?;
        Ok(formatted)
    }

    #[rstest]
    #[case::url("{url}", "http://example.com/hello", "http://example.com/hello")]
    #[case::scheme("{scheme}", "http://example.com/hello", "http")]
    #[case::host("{host}", "http://example.com/hello", "example.com")]
    #[case::port("{port}", "http://example.com:8080/hello", "8080")]
    #[case::default_port("{port}", "http://example.com/hello", "80")]
    #[case::user("{user}", "http://foo:bar@example.com/hello", "foo")]
    #[case::password("{password}", "http://foo:bar@example.com/hello", "bar")]
    #[case::path("{path}", "http://example.com/hello", "/hello")]
    #[case::query("{query}", "http://example.com/hello?x=a", "x=a")]
    #[case::fragment("{fragment}", "http://example.com/hello?x=a#potato", "potato")]
    fn templates(#[case] format: &str, #[case] input_url: &str, #[case] expected: &str) {
        let input_url = Url::parse(input_url).expect("invalid input URL");
        let renderer = UrlRenderer::templated(format);
        let formatted = render_to_string(renderer, &input_url).expect("formatting failed");
        assert_eq!(formatted, expected);
    }

    #[rstest]
    #[case::unknown_key("{other}")]
    #[case::broken_format_close("{other")]
    fn invalid_format(#[case] format: &str) {
        let input_url = Url::parse("http://example.com").expect("invalid input URL");
        let renderer = UrlRenderer::templated(format);
        let result = render_to_string(renderer, &input_url);
        assert!(result.is_err(), "result was {result:?}");
    }
}
