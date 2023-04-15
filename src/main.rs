use std::iter;

use clap::{error::ErrorKind, CommandFactory, Parser};
use trustrl::{parse_url, TransformError, UrlTemplate, UrlTransformation};
use url::Url;

#[derive(Parser)]
struct Cli {
    /// The URL to be used.
    url: String,

    /// The template to be used to render the URL.
    #[clap(short = 't', long, default_value = "{url}")]
    template: String,

    /// Set the URL's scheme.
    #[clap(short = 's', long)]
    scheme: Option<String>,

    /// Set the URL's host.
    #[clap(short = 'H', long)]
    host: Option<String>,

    /// Set the URL's port.
    #[clap(short = 'P', long)]
    port: Option<u16>,

    /// Set the URL's path.
    #[clap(short = 'p', long, group = "paths")]
    path: Option<String>,

    /// Set the URL's user.
    #[clap(short = 'u', long)]
    user: Option<String>,

    /// Set the URL's password.
    #[clap(short = 'S', long)]
    password: Option<String>,

    /// Set the URL's fragment.
    #[clap(short = 'f', long)]
    fragment: Option<String>,

    /// Redirect the URL to a new path.
    #[clap(short = 'r', long, group = "paths")]
    redirect: Option<String>,

    /// Append a new segment at the end of the path.
    #[clap(short = 'a', long, group = "paths")]
    append_path: Option<String>,

    /// Sort query string.
    #[clap(short = 'q', long)]
    sort_query_string: bool,

    /// Clear the query string.
    #[clap(short = 'c', long)]
    clear_query_string: bool,
}

#[cfg(test)]
mod test {
    use super::Cli;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}

fn optional_string(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn build_transformations(cli: &Cli) -> Vec<UrlTransformation> {
    iter::empty()
        .chain(cli.scheme.as_deref().map(UrlTransformation::SetScheme).into_iter())
        .chain(cli.host.as_deref().map(UrlTransformation::SetHost).into_iter())
        .chain(cli.port.map(UrlTransformation::SetPort).into_iter())
        .chain(cli.path.as_deref().map(UrlTransformation::SetPath).into_iter())
        .chain(cli.user.as_deref().map(UrlTransformation::SetUser).into_iter())
        .chain(cli.password.as_deref().map(optional_string).map(UrlTransformation::SetPassword).into_iter())
        .chain(cli.fragment.as_deref().map(optional_string).map(UrlTransformation::SetFragment).into_iter())
        .chain(cli.redirect.as_deref().map(UrlTransformation::Redirect).into_iter())
        .chain(cli.append_path.as_deref().map(UrlTransformation::AppendPath).into_iter())
        .chain(cli.sort_query_string.then_some(UrlTransformation::SortQueryString).into_iter())
        .chain(cli.clear_query_string.then_some(UrlTransformation::ClearQueryString).into_iter())
        .collect()
}

fn transform(mut url: Url, transformations: Vec<UrlTransformation>) -> Result<Url, TransformError> {
    for transformation in transformations {
        url = transformation.apply(url)?
    }
    Ok(url)
}

fn render(url: &Url, template: &UrlTemplate) {
    match template.render(url) {
        Ok(rendered) => {
            println!("{rendered}");
        }
        Err(e) => {
            eprintln!("Template rendering failed: {e}");
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let url = match parse_url(&cli.url) {
        Ok(url) => url,
        Err(e) => {
            let mut cmd = Cli::command();
            cmd.error(ErrorKind::ValueValidation, format!("Invalid URL: {e}")).exit();
        }
    };
    let template = UrlTemplate::new(&cli.template);
    let transformations = build_transformations(&cli);
    match transform(url, transformations) {
        Ok(url) => render(&url, &template),
        Err(e) => eprintln!("Error performing transformations: {e}"),
    };
}
