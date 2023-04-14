//! URL transformations.

use url::{ParseError, Url};

/// A URL transformation.
///
/// This represents a transformation that can be applied to a URL.
#[derive(Clone, Debug)]
pub enum UrlTransformation<'a> {
    /// Set the URL scheme.
    SetScheme(&'a str),

    /// Set the URL host.
    SetHost(&'a str),

    /// Set the URL port.
    SetPort(u16),

    /// Set the URL path.
    SetPath(&'a str),

    /// Set the URL user.
    SetUser(&'a str),

    /// Set the URL password.
    SetPassword(Option<&'a str>),

    /// Set the URL fragment.
    SetFragment(Option<&'a str>),

    /// Redirect to a new path.
    ///
    /// If the provided path is relative, the last segment in the URL path will be replaced with
    /// it.
    ///
    /// If the provided path is absolute, the entire path will be replaced with it.
    Redirect(&'a str),

    /// Append a new segment to the end of the path.
    AppendPath(&'a str),

    /// Append a new query string key/value pair.
    AppendQuery(&'a str, &'a str),
}

impl<'a> UrlTransformation<'a> {
    /// Apply the transformation on the given URL.
    ///
    /// # Example
    ///
    /// ```
    /// # use url::Url;
    /// # use trustrl::UrlTransformation;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let url = Url::parse("http://example.com")?;
    /// let transformation = UrlTransformation::SetScheme("https");
    /// let new_url = transformation.apply(url)?;
    /// assert_eq!(&new_url.to_string(), "https://example.com");
    /// # Ok(())
    /// # }
    /// ```
    pub fn apply(&self, mut url: Url) -> Result<Url, TransformError> {
        use TransformError::*;
        use UrlTransformation::*;
        match *self {
            SetScheme(scheme) => {
                url.set_scheme(scheme).map_err(|_| Transform("scheme"))?;
            }
            SetHost(host) => {
                url.set_host(Some(host)).map_err(|e| Parse("host", e))?;
            }
            SetPort(port) => {
                url.set_port(Some(port)).map_err(|_| Transform("port"))?;
            }
            SetPath(path) => {
                url.set_path(path);
            }
            SetUser(user) => {
                url.set_username(user).map_err(|_| Transform("user"))?;
            }
            SetPassword(password) => {
                url.set_password(password).map_err(|_| Transform("password"))?;
            }
            SetFragment(fragment) => {
                url.set_fragment(fragment);
            }
            Redirect(path) => {
                if path.as_bytes().first() == Some(&b'/') {
                    url.set_path(path)
                } else {
                    let mut segments = url.path_segments_mut().map_err(|_| Transform("redirect"))?;
                    segments.pop();
                    segments.push(path);
                }
            }
            AppendPath(path) => {
                let mut segments = url.path_segments_mut().map_err(|_| Transform("append-path"))?;
                segments.push(path);
                drop(segments);
            }
            AppendQuery(name, value) => {
                url.query_pairs_mut().append_pair(name, value);
            }
        };
        Ok(url)
    }
}

/// An error during the application of a transformation.
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    /// A transformation failed.
    #[error("failed to apply {0} transformation")]
    Transform(&'static str),

    /// Something that we parsed failed. e.g. a hostname.
    #[error("parsing '{0}' failed: {1}")]
    Parse(&'static str, ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use UrlTransformation::*;

    #[rstest]
    #[case::scheme(SetScheme("https"), "http://foo.com", "https://foo.com")]
    #[case::host(SetHost("bar.com"), "http://foo.com", "http://bar.com")]
    #[case::port(SetPort(8080), "http://foo.com", "http://foo.com:8080")]
    #[case::path(SetPath("/potato"), "http://foo.com/bar/zar", "http://foo.com/potato")]
    #[case::user(SetUser("me"), "http://foo.com", "http://me@foo.com")]
    #[case::password(SetPassword(Some("secret")), "http://me@foo.com", "http://me:secret@foo.com")]
    #[case::no_password(SetPassword(None), "http://me:secret@foo.com", "http://me@foo.com")]
    #[case::fragment(SetFragment(Some("needle")), "http://foo.com/hello", "http://foo.com/hello#needle")]
    #[case::no_fragment(SetFragment(None), "http://foo.com/hello#needle", "http://foo.com/hello")]
    #[case::redirect_relative(Redirect("potato"), "http://foo.com/bar/zar", "http://foo.com/bar/potato")]
    #[case::redirect_absolute(Redirect("/potato"), "http://foo.com/bar/zar", "http://foo.com/potato")]
    #[case::append_path(AppendPath("potato"), "http://foo.com/bar", "http://foo.com/bar/potato")]
    #[case::append_path_urlencode(
        AppendPath("potato nuggets"),
        "http://foo.com/bar",
        "http://foo.com/bar/potato%20nuggets"
    )]
    #[case::append_query(AppendQuery("side", "potato"), "http://foo.com/bar", "http://foo.com/bar?side=potato")]
    #[case::append_query_existing(
        AppendQuery("side", "potato"),
        "http://foo.com/bar?q=a",
        "http://foo.com/bar?q=a&side=potato"
    )]
    #[case::append_query_repeated(
        AppendQuery("side", "potato"),
        "http://foo.com/bar?side=nuggets",
        "http://foo.com/bar?side=nuggets&side=potato"
    )]
    fn transformations(#[case] transformation: UrlTransformation, #[case] input_url: &str, #[case] expected_url: &str) {
        let input_url = Url::parse(input_url).expect("invalid input url");
        let expected_url = Url::parse(expected_url).expect("invalid input url");

        let transformed_url = transformation.apply(input_url).expect("transformation failed");
        assert_eq!(transformed_url, expected_url, "failed for {transformation:?}");
    }
}
