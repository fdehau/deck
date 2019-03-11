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
    },
    #[structopt(name = "serve")]
    Serve {
        #[structopt(long = "port", short = "p")]
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
        Command::Build { theme } => {
            // Read input from stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;

            // Render html to stdout
            let options = html::Options {
                theme,
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
