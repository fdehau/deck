use std::fmt;
use std::path::PathBuf;

use pulldown_cmark::{html, Event, Options as MarkdownOptions, Parser, Tag};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{
    start_highlighted_html_snippet, styled_line_to_highlighted_html, IncludeBackground,
};
use syntect::parsing::SyntaxSet;

use crate::error::Error;

const DEFAULT_THEME: &str = "base16-ocean.dark";

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
    pub theme_dirs: Vec<PathBuf>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            title: None,
            theme: None,
            theme_dirs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Renderer {
    syntax_set: SyntaxSet,
    theme: Theme,
    title: Option<String>,
}

impl Renderer {
    pub fn try_new(options: Options) -> Result<Renderer, Error> {
        // Load syntax and theme
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let mut theme_set = ThemeSet::load_defaults();
        for theme_dir in &options.theme_dirs {
            theme_set.add_from_folder(theme_dir)?;
        }
        let theme_name = options.theme.unwrap_or_else(|| DEFAULT_THEME.to_owned());
        let theme = theme_set
            .themes
            .remove(&theme_name)
            .ok_or_else(|| Error::ThemeNotFound)?;
        Ok(Renderer {
            syntax_set,
            theme,
            title: options.title,
        })
    }

    pub fn render(
        &self,
        input: String,
        css: Option<String>,
        js: Option<String>,
    ) -> Result<Output, Error> {
        // Create parser
        let mut opts = MarkdownOptions::empty();
        opts.insert(MarkdownOptions::ENABLE_TABLES);
        let parser = Parser::new_ext(&input, opts);
        let mut in_code_block = false;
        let mut highlighter = None;
        let parser = parser.map(|event| match event {
            Event::Start(Tag::Rule) => {
                Event::Html("</div>\n</div>\n<div class=\"slide\">\n<div class=\"content\">".into())
            }
            Event::Start(Tag::CodeBlock(ref lang)) => {
                in_code_block = true;
                let snippet = start_highlighted_html_snippet(&self.theme);
                if let Some(syntax) = self.syntax_set.find_syntax_by_token(lang) {
                    highlighter = Some(HighlightLines::new(syntax, &self.theme));
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
                        let highlighted = highlighter.highlight(&text, &self.syntax_set);
                        let html =
                            styled_line_to_highlighted_html(&highlighted, IncludeBackground::No);
                        return Event::Html(html.into());
                    }
                }
                Event::Text(text)
            }
            e => e,
        });

        let mut html = String::with_capacity(input.len());
        html::push_html(&mut html, parser);
        html.insert_str(0, "<div class=\"slide\">\n<div class=\"content\">\n");
        html.push_str("</div>\n</div>");

        // Build inline css
        let mut style = include_str!("style.css").to_owned();
        if let Some(ref custom_css) = css {
            style.push_str(custom_css);
        }
        let style = minifier::css::minify(&style).map_err(|s| Error::Minification(s))?;

        // Build inline js
        let mut script = include_str!("script.js").to_owned();
        if let Some(ref custom_js) = js {
            script.push_str(custom_js);
        }
        let script = minifier::js::minify(&script);
        Ok(Output {
            title: self.title.clone(),
            style,
            script,
            body: html,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render() {
        let input = r#"
# Slide 1

This is a **test**

---

# Slide 2

And it should work"#;
        let renderer = Renderer::try_new(Options::default()).expect("Failed to create renderer");
        let output = renderer
            .render(input.into(), None, None)
            .expect("Failed to render");
        assert_eq!(
            r#"<div class="slide">
<div class="content">
<h1>Slide 1</h1>
<p>This is a <strong>test</strong></p>
</div>
</div>
<div class="slide">
<div class="content">
<h1>Slide 2</h1>
<p>And it should work</p>
</div>
</div>"#,
            output.body
        );
    }
}
