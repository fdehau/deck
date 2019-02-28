use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use log::{debug, error, info};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use pulldown_cmark::{html, Event, Options, Parser, Tag};
use structopt::StructOpt;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::{
    start_highlighted_html_snippet, styled_line_to_highlighted_html, IncludeBackground,
};
use syntect::parsing::SyntaxSet;
use warp::ws::Message;
use warp::{Filter, Future, Stream};

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

type Users = Arc<Mutex<HashMap<usize, futures::sync::mpsc::UnboundedSender<Message>>>>;

struct HTMLOutput {
    title: String,
    style: String,
    script: String,
    body: String,
}

impl fmt::Display for HTMLOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<html><head><meta charset=\"utf-8\"><title>{}</title><style>{}</style><script type=\"text/javascript\">{}</script></head><body>{}</body></html>",
            self.title, self.style, self.script, self.body
        )
    }
}

fn render_html(input: String) -> HTMLOutput {
    // Load syntax and theme
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let theme = &theme_set.themes["base16-ocean.dark"];

    // Create parser
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(&input, opts);
    let mut in_code_block = false;
    let mut highlighter = None;
    let parser = parser.map(|event| match event {
        Event::Start(Tag::Rule) => Event::Html(Cow::Borrowed("</div></div><div class=\"slide\"><div class=\"content\">")),
        Event::Start(Tag::CodeBlock(ref lang)) => {
            in_code_block = true;
            let snippet = start_highlighted_html_snippet(theme);
            if let Some(syntax) = syntax_set.find_syntax_by_token(lang) {
                highlighter = Some(HighlightLines::new(syntax, theme));
            }
            Event::Html(Cow::Owned(snippet.0))
        }
        Event::End(Tag::CodeBlock(_)) => {
            highlighter = None;
            Event::Html(Cow::Borrowed("</pre>"))
        }
        Event::Text(text) => {
            if in_code_block {
                if let Some(ref mut highlighter) = highlighter {
                    let highlighted = highlighter.highlight(&text, &syntax_set);
                    let html = styled_line_to_highlighted_html(&highlighted, IncludeBackground::No);
                    return Event::Html(Cow::Owned(html));
                }
            }
            Event::Text(text)
        }
        e => e,
    });

    // Now we send this new vector of events off to be transformed into HTML
    let mut html = String::with_capacity(input.len());
    html::push_html(&mut html, parser);
    html.insert_str(0, "<div class=\"slide\"><div class=\"content\">");
    html.push_str("</div/></div>");
    HTMLOutput {
        title: "Slides".to_owned(),
        style: include_str!("style.css").to_owned(),
        script: include_str!("script.js").to_owned(),
        body: html,
    }
}

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "build")]
    Build,
    #[structopt(name = "serve")]
    Serve {
        #[structopt(parse(from_os_str))]
        input: PathBuf,
        #[structopt(long = "watch", short = "w")]
        watch: bool,
    },
}

struct WatcherThread {
    watcher: RecommendedWatcher,
    handle: thread::JoinHandle<()>,
}

fn watch_file(users: Users, input: PathBuf) -> Result<WatcherThread, Box<dyn Error>> {
    let (tx, rx) = mpsc::channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(1))?;
    watcher.watch(input, RecursiveMode::NonRecursive)?;
    let notify_users = users.clone();
    let handle = thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(event) => match event {
                    DebouncedEvent::Write(path) => {
                        info!("tag=watch_event path={}", path.display());
                        for (&id, tx) in notify_users.lock().unwrap().iter() {
                            debug!("tag=reload path={} id={}", path.display(), id);
                            match tx.unbounded_send(Message::text("reload")) {
                                Ok(()) => (),
                                Err(_disconnected) => {
                                    // The tx is disconnected, our `user_disconnected` code
                                    // should be happening in another task, nothing more to
                                    // do here.
                                }
                            }
                        }
                    }
                    _ => {}
                },
                Err(err) => error!("tag=watch_error, msg=\"{}\"", err),
            }
        }
    });
    Ok(WatcherThread { watcher, handle })
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::from_args();

    pretty_env_logger::init();

    match cli.cmd {
        Command::Build => {
            // Read input from stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;

            // Render html to stdout
            let html = render_html(input);
            print!("{}", html);
            Ok(())
        }
        Command::Serve { input, watch } => {
            // Initialize websockets users
            let users: Users = Arc::new(Mutex::new(HashMap::new()));

            let _watcher_thread = if watch {
                Some(watch_file(users.clone(), input.clone())?)
            } else {
                None
            };

            // Setup routes
            let slides = {
                let slides_index = warp::path("slides").and(warp::path::end());
                warp::get2().and(slides_index).and_then(move || {
                    let input = input.clone();
                    match File::open(input) {
                        Ok(file) => {
                            let mut buf_reader = BufReader::new(file);
                            let mut content = String::new();
                            match buf_reader.read_to_string(&mut content) {
                                Ok(_) => {
                                    let html = render_html(content);
                                    Ok(warp::reply::html(format!("{}", html)))
                                }
                                Err(err) => {
                                    error!("tag=io_error msg=\"{}\"", err);
                                    Err(warp::reject::server_error())
                                }
                            }
                        }
                        Err(err) => {
                            error!("tag=fs_error msg=\"{}\"", err);
                            Err(warp::reject::server_error())
                        }
                    }
                })
            };
            let ws = {
                let users = warp::any().map(move || users.clone());
                warp::path("ws").and(warp::ws2()).and(users).map(
                    |ws: warp::ws::Ws2, users: Users| {
                        ws.on_upgrade(move |websocket| {
                            let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
                            info!("tag=ws_connect id={}", id);
                            let (ws_tx, ws_rx) = websocket.split();
                            let (tx, rx) = futures::sync::mpsc::unbounded();
                            warp::spawn(
                                rx.map_err(|()| -> warp::Error {
                                    unreachable!("unbounded rx never errors")
                                })
                                .forward(ws_tx)
                                .map(|_tx_rx| ())
                                .map_err(|err| error!("tag=ws_error msg=\"{}\"", err)),
                            );
                            users.lock().unwrap().insert(id, tx);
                            ws_rx
                                .for_each(|_msg| Ok(()))
                                .then(move |result| {
                                    info!("tag=ws_disconnect id={}", id);
                                    users.lock().unwrap().remove(&id);
                                    result
                                })
                                .map_err(|err| {
                                    error!("tag=ws_error msg=\"{}\"", err);
                                })
                        })
                    },
                )
            };
            let routes = slides.or(ws);

            let addr: SocketAddr = ([127, 0, 0, 1], 3030).into();
            info!("tag=server_start addr={}", addr);

            // Start server
            warp::serve(routes).run(addr);
            Ok(())
        }
    }
}
