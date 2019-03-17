[![Build Status](https://travis-ci.com/fdehau/deck.svg?branch=master)](https://travis-ci.com/fdehau/deck)
[![Crate Status](https://img.shields.io/crates/v/deck.svg)](https://crates.io/crates/deck)
[![Docs Status](https://docs.rs/deck/badge.svg)](https://docs.rs/crate/deck/)

# Deck

Deck is a command line tool that generates HTML presentations from Markdown
documents.

## Usage

### Build

```
deck build < slides.md > slides.html
```

### Serve

```
deck serve slides.md -w -p 8000
```

## Todos

* Load local syntect themes

Maybe:

* Speaker notes
* Timer
