use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use structopt::StructOpt;

use crate::error::Error;

mod error;
mod html;
mod server;

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(long = "verbose", short = "v")]
    verbose: bool,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "build")]
    Build {
        #[structopt(long = "title")]
        title: Option<String>,
        #[structopt(long = "theme")]
        theme: Option<String>,
        #[structopt(long = "theme-dir")]
        theme_dirs: Vec<PathBuf>,
        #[structopt(long = "css")]
        css: Option<PathBuf>,
        #[structopt(long = "js")]
        js: Option<PathBuf>,
    },
    #[structopt(name = "serve")]
    Serve {
        #[structopt(long = "port", short = "p", default_value = "8000")]
        port: u16,
        #[structopt(parse(from_os_str))]
        input: PathBuf,
        #[structopt(long = "watch", short = "w")]
        watch: bool,
        #[structopt(long = "theme")]
        theme: Option<String>,
        #[structopt(long = "theme-dir")]
        theme_dirs: Vec<PathBuf>,
        #[structopt(long = "css")]
        css: Option<PathBuf>,
        #[structopt(long = "js")]
        js: Option<PathBuf>,
    },
}

fn main() -> Result<(), Error> {
    let cli = Cli::from_args();

    pretty_env_logger::formatted_builder()
        .filter_module(
            "deck",
            if cli.verbose {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Warn
            },
        )
        .init();

    match cli.cmd {
        Command::Build {
            theme,
            title,
            css,
            js,
            theme_dirs,
        } => {
            // Read input from stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;

            let css = if let Some(path) = css {
                let s = fs::read_to_string(path)?;
                Some(s)
            } else {
                None
            };

            let js = if let Some(path) = js {
                let s = fs::read_to_string(path)?;
                Some(s)
            } else {
                None
            };

            // Render html to stdout
            let options = html::Options {
                title,
                theme,
                css,
                js,
                theme_dirs,
            };

            let renderer = html::Renderer::try_new(options)?;
            let html = renderer.render(input)?;
            print!("{}", html);
        }
        Command::Serve {
            port,
            input,
            watch,
            theme,
            theme_dirs,
            css,
            js,
        } => {
            let config = server::Config {
                port,
                watch,
                input,
                theme,
                theme_dirs,
                css,
                js,
            };
            server::start(config)?;
        }
    }
    Ok(())
}
