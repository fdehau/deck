use std::fmt;
use std::io;

use pulldown_cmark::{html, Event, Options as MarkdownOptions, Parser, Tag};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::{
    start_highlighted_html_snippet, styled_line_to_highlighted_html, IncludeBackground,
};
use syntect::parsing::SyntaxSet;

use crate::error::Error;

const DEFAULT_THEME: &'static str = "base16-ocean.dark";

pub struct Output {
    title: Option<String>,
    style: String,
    script: String,
    body: String,
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "<html>")?;
        writeln!(f, "<head>")?;

        // Meta
        writeln!(f, "<meta charset=\"utf-8\">")?;
        if let Some(ref title) = self.title {
            writeln!(f, "<title>{}</title>", title)?;
        }

        // Style
        writeln!(f, "<style>")?;
        writeln!(f, "{}", self.style)?;
        writeln!(f, "</style>")?;
        writeln!(f, "<script type=\"text/javascript\">")?;
        writeln!(f, "{}", self.script)?;
        writeln!(f, "</script>")?;

        writeln!(f, "<body>")?;
        writeln!(f, "{}", self.body)?;
        writeln!(f, "</body>")?;

        writeln!(f, "</head>")?;

        writeln!(f, "</html>")
    }
}

pub struct Options {
    pub title: Option<String>,
    pub theme: Option<String>,
    pub css: Option<String>,
    pub js: Option<String>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            title: None,
            theme: None,
            css: None,
            js: None,
        }
    }
}

pub fn render(input: String, options: Options) -> Result<Output, Error> {
    // Load syntax and theme
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let theme_name = options.theme.unwrap_or(DEFAULT_THEME.to_owned());
    let theme = &theme_set.themes.get(&theme_name).ok_or_else(|| {
        Error::SyntaxHightlighting(syntect::LoadingError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Theme not found",
        )))
    })?;

    // Create parser
    let mut opts = MarkdownOptions::empty();
    opts.insert(MarkdownOptions::ENABLE_TABLES);
    let parser = Parser::new_ext(&input, opts);
    let mut in_code_block = false;
    let mut highlighter = None;
    let parser = parser.map(|event| match event {
        Event::Start(Tag::Rule) => {
            Event::Html("</div></div><div class=\"slide\"><div class=\"content\">".into())
        }
        Event::Start(Tag::CodeBlock(ref lang)) => {
            in_code_block = true;
            let snippet = start_highlighted_html_snippet(theme);
            if let Some(syntax) = syntax_set.find_syntax_by_token(lang) {
                highlighter = Some(HighlightLines::new(syntax, theme));
            }
            Event::Html(snippet.0.into())
        }
        Event::End(Tag::CodeBlock(_)) => {
            highlighter = None;
            Event::Html("</pre>".into())
        }
        Event::Text(text) => {
            if in_code_block {
                if let Some(ref mut highlighter) = highlighter {
                    let highlighted = highlighter.highlight(&text, &syntax_set);
                    let html = styled_line_to_highlighted_html(&highlighted, IncludeBackground::No);
                    return Event::Html(html.into());
                }
            }
            Event::Text(text)
        }
        e => e,
    });

    let mut html = String::with_capacity(input.len());
    html::push_html(&mut html, parser);
    html.insert_str(0, "<div class=\"slide\"><div class=\"content\">");
    html.push_str("</div></div>");

    // Build inline css
    let mut style = include_str!("style.css").to_owned();
    if let Some(custom_css) = options.css {
        style.push_str(&custom_css);
    }
    let style = minifier::css::minify(&style).map_err(|s| Error::Minification(s))?;

    // Build inline js
    let mut script = include_str!("script.js").to_owned();
    if let Some(custom_js) = options.js {
        script.push_str(&custom_js);
    }
    let script = minifier::js::minify(&script);
    Ok(Output {
        title: options.title,
        style,
        script,
        body: html,
    })
}
