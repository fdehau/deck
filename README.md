[![Build Status](https://travis-ci.com/fdehau/deck.svg?branch=master)](https://travis-ci.com/fdehau/deck)
[![Crate Status](https://img.shields.io/crates/v/deck.svg)](https://crates.io/crates/deck)
[![Docs Status](https://docs.rs/deck/badge.svg)](https://docs.rs/crate/deck/)

# Deck

Deck is a command line tool that generates HTML presentations from Markdown
documents.

## Input

Slides are written in Markdown. Horizontal rules (`---`) are used to separate
each slide.

## Usage

### Build

A Markdown file can be converted to an HTML presentation with a single command
in a single file. By default, the generated HTML contains some inline CSS and
Javascript to render the slides correctly. If you wish to customize the output
a bit more you pass additional CSS and Javascript files using either the
`--css` and `--js` options. The resulting document can be open in most modern
browsers.

```
deck build < slides.md > slides.html
```

### Serve

You also have the possibility to serve Markdown slides using the built-in
server. The following command makes the presentation available at
`http://localhost:8000/slides`:

```
deck serve slides.md -p 8000
```

When writing your presentation, it might come in handy to see the resulting
HTML presentation evolves as you write. Adding `-w` to the previous command
and `?watch=true` to the previous URL will ensure that the web page is reloaded
as soon as either the Markdown slides, the custom css or the customm js are
modified.

## Syntax highlighting

Syntax highlighting can be customized in various ways. First, both
`build` and `serve` commands allow you to choose a different theme using
the `--theme` option. By default only a handful of themes are available
as listed [here](https://docs.rs/syntect/latest/syntect/highlighting/struct.ThemeSet.html#method.load_defaults).

```
deck build --theme InspiredGitHub < slides.md > slides.html
```

In addition, `syntect`, the crate doing all the heavy lifting of highlighting
the code, is able to load all TextMate and Sublime Text `.tmTheme` color
schemes. In order to load a local theme, you must first add its directory
to the list of paths where the binary will look for compatible themes and
then select it using `--theme`. Given that the `gruvbox.tmTheme` is under
the directory `./themes` the command invocation could look like:

```
deck build --theme-dir ./themes --theme gruvbox < slides.md > slides.html
```

## Todos

* Speaker notes
* Timer
