use clap::{error::ErrorKind, Args, CommandFactory, Parser};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    iter,
    path::{Path, PathBuf},
    process::exit,
};
use trustrl::{parse_url, TransformError, UrlRenderer, UrlTransformation};
use url::Url;

#[derive(Parser)]
struct Cli {
    #[clap(flatten)]
    input: Input,

    /// The template to be used to render the URL.
    #[clap(short = 't', long, default_value = "{url}", group = "outputs")]
    template: String,

    /// Output URLs in JSON format.
    #[clap(short = 'j', long = "to-json", group = "outputs")]
    output_json: bool,

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

#[derive(Args)]
#[group(required = true, multiple = false)]
struct Input {
    /// The URL to be used.
    url: Option<String>,

    /// A path to a list of URLs to process.
    #[clap(long)]
    urls_file_path: Option<PathBuf>,
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

enum RenderMode {
    Single,
    JsonList { count: usize },
}

struct RenderContext<'a, W: Write> {
    renderer: UrlRenderer<'a>,
    writer: W,
    mode: RenderMode,
}

impl<'a, W: Write> RenderContext<'a, W> {
    fn new_single_line(renderer: UrlRenderer<'a>, writer: W) -> Self {
        Self { renderer, writer, mode: RenderMode::Single }
    }

    fn new_json_list(renderer: UrlRenderer<'a>, writer: W) -> Self {
        Self { renderer, writer, mode: RenderMode::JsonList { count: 0 } }
    }

    fn render(&mut self, url: &Url) -> Result<(), Box<dyn std::error::Error>> {
        use RenderMode::JsonList;
        match self.mode {
            JsonList { count: 0 } => write!(self.writer, "[")?,
            JsonList { .. } => write!(self.writer, ", ")?,
            _ => (),
        };
        if matches!(self.mode, RenderMode::JsonList { count: 0 }) {}
        self.renderer.render(url, &mut self.writer)?;
        if let RenderMode::JsonList { count } = &mut self.mode {
            *count += 1;
        } else {
            writeln!(self.writer)?;
        }
        Ok(())
    }
}

impl<'a, W: Write> Drop for RenderContext<'a, W> {
    fn drop(&mut self) {
        if matches!(self.mode, RenderMode::JsonList { .. }) {
            let _ = writeln!(self.writer, "]");
        }
    }
}

struct Processor<'a, W: Write> {
    context: RenderContext<'a, W>,
    transformations: Vec<UrlTransformation<'a>>,
}

impl<'a, W: Write> Processor<'a, W> {
    fn new(context: RenderContext<'a, W>, transformations: Vec<UrlTransformation<'a>>) -> Self {
        Self { context, transformations }
    }

    fn process_url(&mut self, url: &str) {
        let url = match parse_url(url) {
            Ok(url) => url,
            Err(e) => {
                let mut cmd = Cli::command();
                cmd.error(ErrorKind::ValueValidation, format!("Invalid URL: {e}")).exit();
            }
        };
        match self.transform(url) {
            Ok(url) => self.render(&url),
            Err(e) => eprintln!("Error performing transformations: {e}"),
        };
    }

    fn process_url_file(&mut self, path: &Path) {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                let mut cmd = Cli::command();
                cmd.error(ErrorKind::ValueValidation, format!("Invalid URL file path: {e}")).exit();
            }
        };
        let file = BufReader::new(file);
        for line in file.lines() {
            match line {
                Ok(line) => self.process_url(&line),
                Err(e) => {
                    eprintln!("Failed to read file: {e}");
                    exit(1);
                }
            };
        }
    }

    fn transform(&self, mut url: Url) -> Result<Url, TransformError> {
        for transformation in &self.transformations {
            url = transformation.apply(url)?
        }
        Ok(url)
    }

    fn render(&mut self, url: &Url) {
        if let Err(e) = self.context.render(url) {
            eprintln!("Rendering failed: {e}");
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let renderer = match cli.output_json {
        true => UrlRenderer::json(),
        false => UrlRenderer::templated(&cli.template),
    };
    let transformations = build_transformations(&cli);
    let stdout = BufWriter::new(io::stdout().lock());
    let render_json_list = cli.output_json && cli.input.urls_file_path.is_some();
    let context = match render_json_list {
        true => RenderContext::new_json_list(renderer, stdout),
        false => RenderContext::new_single_line(renderer, stdout),
    };
    let mut processor = Processor::new(context, transformations);
    match (&cli.input.url, &cli.input.urls_file_path) {
        (Some(url), _) => processor.process_url(url),
        (None, Some(path)) => processor.process_url_file(path),
        _ => unreachable!(),
    };
}
