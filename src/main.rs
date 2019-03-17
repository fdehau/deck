use std::fs::File;
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
        #[structopt(long = "theme")]
        theme: Option<String>,
        #[structopt(long = "title")]
        title: Option<String>,
        #[structopt(long = "css")]
        css: Option<PathBuf>,
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
        Command::Build { theme, title, css } => {
            // Read input from stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;

            let css = if let Some(path) = css {
                let mut s = String::new();
                let mut file = File::open(path)?;
                file.read_to_string(&mut s)?;
                Some(s)
            } else {
                None
            };

            // Render html to stdout
            let options = html::Options {
                theme,
                css,
                title,
                ..html::Options::default()
            };
            let html = html::render(input, options)?;
            print!("{}", html);
        }
        Command::Serve {
            port,
            input,
            watch,
            theme,
        } => {
            let config = server::Config {
                port,
                watch,
                input,
                theme,
            };
            server::start(config)?;
        }
    }
    Ok(())
}
