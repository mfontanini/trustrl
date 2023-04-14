//! URL templates.

use runtime_format::{FormatArgs, FormatKey, FormatKeyError};
use std::io::{BufWriter, IntoInnerError, Write};
use url::Url;

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
    ///
    /// # Example
    ///
    /// ```
    /// # use url::Url;
    /// # use trustrl::UrlTemplate;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let url = Url::parse("https://example.com/foo")?;
    /// let template = UrlTemplate::new("scheme is {scheme}, path is {path}");
    /// assert_eq!(template.render(&url)?, "scheme is https, path is /foo");
    /// # Ok(())
    /// # }
    /// ````
    pub fn render(&self, url: &Url) -> Result<String, RenderError> {
        let formatter = UrlFormatter { url };
        let args = FormatArgs::new(self.format, &formatter);
        let mut writer = BufWriter::new(Vec::new());
        write!(writer, "{args}")?;

        let buffer = writer.into_inner().map_err(IntoInnerError::into_error)?;
        let formatted = String::from_utf8(buffer).map_err(|_| RenderError::NotUtf8)?;
        Ok(formatted)
    }
}

/// An error during the rendering of a URL.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// An IO error during the formatting.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// The resulting string is not valid utf8.
    #[error("template produced non-utf8 string")]
    NotUtf8,
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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
        let template = UrlTemplate::new(format);
        let formatted = template.render(&input_url).expect("formatting failed");
        assert_eq!(formatted, expected);
    }

    #[rstest]
    #[case::unknown_key("{other}")]
    #[case::broken_format_close("{other")]
    fn invalid_format(#[case] format: &str) {
        let input_url = Url::parse("http://example.com").expect("invalid input URL");
        let template = UrlTemplate::new(format);
        let result = template.render(&input_url);
        assert!(result.is_err(), "result was {result:?}");
    }
}
