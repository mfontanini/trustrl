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
    AppendQueryString(&'a str, &'a str),

    /// Sort the query string.
    SortQueryString,

    /// Reset the query string.
    ClearQueryString,
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
            SetScheme(scheme) => url = Self::set_scheme(url, scheme)?,
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
            AppendQueryString(name, value) => {
                url.query_pairs_mut().append_pair(name, value);
            }
            SortQueryString => url = Self::sort_query_string(url),
            ClearQueryString => {
                url.set_query(None);
            }
        };
        Ok(url)
    }

    fn set_scheme(mut url: Url, scheme: &str) -> Result<Url, TransformError> {
        if url.set_scheme(scheme).is_ok() {
            return Ok(url);
        }
        // `Url::set_scheme` is very picky about which scheme transitions are valid. So if the initial
        // attempt to set the scheme fails, we replace it by hand and re-parse the URL.
        use TransformError::Transform;
        let url = url.to_string();
        let rest = url.split_once(':').ok_or(Transform("scheme"))?.1;
        let url = format!("{scheme}:{rest}");
        Url::parse(&url).map_err(|_| Transform("scheme"))
    }

    fn sort_query_string(mut url: Url) -> Url {
        let mut key_values: Vec<_> = url.query_pairs().into_owned().collect();
        // This otherwise creates an empty query string.
        if key_values.is_empty() {
            return url;
        }
        key_values.sort();
        url.query_pairs_mut().clear().extend_pairs(key_values.into_iter()).finish();
        url
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
    #[case::scheme(SetScheme("https"), "http://foo.com", "https://foo.com/")]
    #[case::scheme_to_other(SetScheme("potato"), "http://foo.com", "potato://foo.com/")]
    #[case::host(SetHost("bar.com"), "http://foo.com", "http://bar.com/")]
    #[case::port(SetPort(8080), "http://foo.com", "http://foo.com:8080/")]
    #[case::path(SetPath("/potato"), "http://foo.com/bar/zar", "http://foo.com/potato")]
    #[case::user(SetUser("me"), "http://foo.com", "http://me@foo.com/")]
    #[case::password(SetPassword(Some("secret")), "http://me@foo.com", "http://me:secret@foo.com/")]
    #[case::no_password(SetPassword(None), "http://me:secret@foo.com", "http://me@foo.com/")]
    #[case::fragment(SetFragment(Some("needle")), "http://foo.com/hello", "http://foo.com/hello#needle")]
    #[case::no_fragment(SetFragment(None), "http://foo.com/hello#needle", "http://foo.com/hello")]
    #[case::no_fragment(ClearQueryString, "http://foo.com/hello?a=1&b=2#id", "http://foo.com/hello#id")]
    #[case::redirect_relative(Redirect("potato"), "http://foo.com/bar/zar", "http://foo.com/bar/potato")]
    #[case::redirect_absolute(Redirect("/potato"), "http://foo.com/bar/zar", "http://foo.com/potato")]
    #[case::append_path(AppendPath("potato"), "http://foo.com/bar", "http://foo.com/bar/potato")]
    #[case::append_path_urlencode(
        AppendPath("potato nuggets"),
        "http://foo.com/bar",
        "http://foo.com/bar/potato%20nuggets"
    )]
    #[case::append_query(AppendQueryString("side", "potato"), "http://foo.com/bar", "http://foo.com/bar?side=potato")]
    #[case::append_query_existing(
        AppendQueryString("side", "potato"),
        "http://foo.com/bar?q=a",
        "http://foo.com/bar?q=a&side=potato"
    )]
    #[case::append_query_repeated(
        AppendQueryString("side", "potato"),
        "http://foo.com/bar?side=nuggets",
        "http://foo.com/bar?side=nuggets&side=potato"
    )]
    #[case::sort_query_string(SortQueryString, "http://foo.com/bar?b=1&a=2&c=3", "http://foo.com/bar?a=2&b=1&c=3")]
    #[case::sort_empty_query_string(SortQueryString, "http://foo.com/", "http://foo.com/")]
    fn transformations(#[case] transformation: UrlTransformation, #[case] input_url: &str, #[case] expected_url: &str) {
        let input_url = Url::parse(input_url).expect("invalid input url");

        let transformed_url = transformation.apply(input_url).expect("transformation failed");
        assert_eq!(transformed_url.to_string(), expected_url, "failed for {transformation:?}");
    }
}
