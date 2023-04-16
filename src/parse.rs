//! Parsing utilities.

use std::borrow::Cow;
use url::Url;

/// Parse a URL.
pub fn parse_url(url: &str) -> Result<Url, UrlParseError> {
    let output = match Url::parse(url) {
        // If we either have a cannot-be-a-base or we failed parsing, attempt to add http as the
        // scheme.
        Ok(url) if url.cannot_be_a_base() => Url::parse(&format!("http://{url}")),
        // Attempt to prepend "http://" unless the URL starts with a slash as that would allow
        // something like "/foo" to be a valid URL.
        Err(url::ParseError::RelativeUrlWithoutBase) if !url.starts_with('/') => Url::parse(&format!("http://{url}")),
        other => other,
    };
    if let Ok(url) = &output {
        // If we still have a cannot-be-a-base here we're done.
        if url.cannot_be_a_base() {
            return Err(UrlParseError("unsupported URL".into()));
        }
    }
    match output {
        Ok(mut url) => {
            if url.scheme().is_empty() {
                // Based on docs this is the only scenario we can hit here but we already validated
                // it is a base.
                url.set_scheme("http").expect("cannot-be-a-base");
            }
            Ok(url)
        }
        Err(e) => Err(UrlParseError(e.to_string().into())),
    }
}

/// An error during the parsing of a URL.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct UrlParseError(Cow<'static, str>);

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::scheme("https://foo.com", "https://foo.com/")]
    #[case::no_scheme("foo.com", "http://foo.com/")]
    #[case::no_scheme_and_user("user@foo.com", "http://user@foo.com/")]
    #[case::no_scheme_and_password(":pass@foo.com", "http://:pass@foo.com/")]
    #[case::no_scheme_and_credentials("user:pass@foo.com", "http://user:pass@foo.com/")]
    fn url_parse_success(#[case] input_url: &str, #[case] expected_url: &str) {
        let url = parse_url(input_url).expect("parse failed");
        assert_eq!(url.to_string(), expected_url);
    }

    #[rstest]
    #[case::data("data:text/plain,Hello?World#")]
    #[case::slash("/foo")]
    fn url_parse_failure(#[case] input_url: &str) {
        let result = parse_url(input_url);
        assert!(result.is_err(), "result was {result:?}");
    }
}
