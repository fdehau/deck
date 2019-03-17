use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, info};
use serde::Serialize;
use warp::ws::Message;
use warp::{Filter, Future, Stream};

use crate::error::Error;
use crate::html;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum Event {
    Reload,
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
type Users = Arc<Mutex<HashMap<usize, futures::sync::mpsc::UnboundedSender<Message>>>>;

fn watch_files<P>(files: &[P], users: Users) -> Result<impl Future<Item = (), Error = ()>, Error>
where
    P: AsRef<Path>,
{
    let mut inotify = Inotify::init()?;
    for file in files {
        inotify.add_watch(file, WatchMask::MODIFY)?;
    }
    let stream = inotify
        .event_stream(vec![0; 1024])
        .for_each(move |event| {
            if event.mask.contains(EventMask::MODIFY) {
                let text = serde_json::to_string(&Event::Reload)?;
                for (&id, tx) in users.lock().unwrap().iter() {
                    debug!("Reloading user, user_id={}", id);
                    tx.unbounded_send(Message::text(text.clone())).ok();
                }
            }
            Ok(())
        })
        .then(|result| {
            debug!("Filesystem event stream closed");
            result
        })
        .map_err(|err| error!("Filesystem event stream encountered an error: {}", err));
    Ok(stream)
}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub watch: bool,
    pub input: PathBuf,
    pub theme: Option<String>,
    pub css: Option<PathBuf>,
    pub js: Option<PathBuf>,
}

fn get_slides(config: Config) -> Result<impl warp::Reply, warp::Rejection> {
    let markdown =
        fs::read_to_string(config.input).map_err(|err| warp::reject::custom(Error::Io(err)))?;
    let css = if let Some(path) = config.css {
        let s = fs::read_to_string(path).map_err(|err| warp::reject::custom(Error::Io(err)))?;
        Some(s)
    } else {
        None
    };
    let js = if let Some(path) = config.js {
        let s = fs::read_to_string(path).map_err(|err| warp::reject::custom(Error::Io(err)))?;
        Some(s)
    } else {
        None
    };
    let options = html::Options {
        theme: config.theme,
        css,
        js,
        ..html::Options::default()
    };
    let html = html::render(markdown, options).map_err(|err| warp::reject::custom(err))?;
    Ok(warp::reply::html(format!("{}", html)))
}

const ERROR_MESSAGE: &'static str = r#"
<html>
<body>
    <h1>Deck encountered an expected error</h1>
    <p>Check the server logs</p>
</body>
</html>
"#;

fn customize_error(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(ref err) = err.find_cause::<Error>() {
        error!("{}", err);
        Ok(warp::reply::with_status(
            warp::reply::html(ERROR_MESSAGE),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else {
        // Could be a NOT_FOUND, or METHOD_NOT_ALLOWED... here we just
        // let warp use its default rendering.
        Err(err)
    }
}

pub fn start(config: Config) -> Result<(), Error> {
    let port = config.port;

    let users = Arc::new(Mutex::new(HashMap::new()));

    // Setup routes
    let slides = {
        let config = config.clone();
        let config = warp::any().map(move || config.clone());
        let slides_index = warp::path("slides").and(warp::path::end());
        warp::get2()
            .and(slides_index)
            .and(config.clone())
            .and_then(get_slides)
    };

    let ws = {
        let users = users.clone();
        let users = warp::any().map(move || users.clone());
        warp::path("ws")
            .and(warp::ws2())
            .and(users)
            .map(|ws: warp::ws::Ws2, users: Users| {
                ws.on_upgrade(move |websocket| {
                    let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
                    debug!("User connected, user_id={}", id);
                    let (ws_tx, ws_rx) = websocket.split();
                    let (tx, rx) = futures::sync::mpsc::unbounded();
                    warp::spawn(
                        rx.map_err(|()| -> warp::Error {
                            unreachable!("unbounded rx never errors")
                        })
                        .forward(ws_tx)
                        .map(|_tx_rx| ())
                        .map_err(|err| error!("Failed transmit message from unbounded channel to websocket stream: {}", err)),
                    );
                    users.lock().unwrap().insert(id, tx);
                    ws_rx
                        .for_each(|_msg| Ok(()))
                        .then(move |result| {
                            debug!("User disconnected, user_id={}", id);
                            users.lock().unwrap().remove(&id);
                            result
                        })
                        .map_err(|err| {
                            error!("Failed communication on the websocket stream: {}", err);
                        })
                })
            })
    };
    let routes = slides.or(ws).recover(customize_error);

    // Configure server
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let server = warp::serve(routes).bind(addr);
    info!("Starting server on {}", addr);

    if config.watch {
        info!("Watching {} for changes", config.input.to_string_lossy());
        let mut files = vec![config.input];
        if let Some(css) = config.css {
            files.push(css.clone());
        }
        if let Some(js) = config.js {
            files.push(js.clone());
        }
        let f = watch_files(&files, users)?;
        tokio::run(server.join(f).map(|_| ()));
    } else {
        tokio::run(server)
    }

    Ok(())
}
