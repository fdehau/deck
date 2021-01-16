use crate::{error::Error, html};
use futures::{FutureExt, StreamExt};
use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, info};
use serde::Serialize;
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::{
    fs,
    sync::{mpsc, Mutex},
};
use warp::{
    reject,
    ws::{Message, WebSocket},
    Filter,
};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum Event {
    Reload,
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
type Users = Arc<Mutex<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;

async fn watch_files<P>(files: Vec<P>, users: Users) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let mut inotify = Inotify::init()?;
    for file in files {
        inotify.add_watch(file, WatchMask::MODIFY)?;
    }
    let mut buffer = [0; 32];
    let mut stream = inotify.event_stream(&mut buffer)?;
    while let Some(res) = stream.next().await {
        let event = res?;
        if event.mask.contains(EventMask::MODIFY) {
            let text = serde_json::to_string(&Event::Reload)?;
            for (&id, tx) in users.lock().await.iter() {
                debug!("Reloading user, user_id={}", id);
                tx.send(Ok(Message::text(text.clone()))).ok();
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub watch: bool,
    pub input: PathBuf,
    pub theme: Option<String>,
    pub theme_dirs: Vec<PathBuf>,
    pub css: Option<PathBuf>,
    pub js: Option<PathBuf>,
}

struct Paths {
    input: PathBuf,
    css: Option<PathBuf>,
    js: Option<PathBuf>,
}

fn convert_error<E: Into<Error>>(err: E) -> warp::Rejection {
    reject::custom(err.into())
}

async fn get_slides(
    paths: Arc<Paths>,
    renderer: Arc<html::Renderer>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let css = if let Some(ref path) = paths.css {
        let s = fs::read_to_string(path).await.map_err(convert_error)?;
        Some(s)
    } else {
        None
    };
    let js = if let Some(ref path) = paths.js {
        let s = fs::read_to_string(path).await.map_err(convert_error)?;
        Some(s)
    } else {
        None
    };
    let markdown = fs::read_to_string(&paths.input)
        .await
        .map_err(convert_error)?;
    let html = renderer.render(markdown, css, js).map_err(convert_error)?;
    Ok(warp::reply::html(format!("{}", html)))
}

const ERROR_MESSAGE: &str = r#"
<html>
<body>
    <h1>Deck encountered an expected error</h1>
    <p>Check the server logs</p>
</body>
</html>
"#;

async fn customize_error(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(ref err) = err.find::<Error>() {
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

async fn handle_ws(ws: WebSocket, users: Users) -> Result<(), Box<dyn std::error::Error>> {
    let user_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(ws_tx).map(move |res| {
        if let Err(e) = res {
            error!(
                "Failed to send over a websocket, user_id: {}, error: {}",
                user_id, e
            )
        }
    }));

    {
        debug!("User connected, user_id: {}", user_id);
        users.lock().await.insert(user_id, tx);
    }

    while let Some(res) = ws_rx.next().await {
        let msg = res?;
        debug!(
            "Message received from user, user_id: {}, msg: {:?}",
            user_id, msg
        );
    }

    {
        debug!("User disconnected, user_id: {}", user_id);
        users.lock().await.remove(&user_id);
    }

    Ok(())
}

pub async fn start(config: Config) -> Result<(), Error> {
    let port = config.port;

    let users = Arc::new(Mutex::new(HashMap::new()));

    // Setup routes
    let slides = {
        let options = html::Options {
            theme: config.theme,
            theme_dirs: config.theme_dirs,
            ..html::Options::default()
        };
        let renderer = {
            let r = html::Renderer::try_new(options)?;
            Arc::new(r)
        };
        let paths = {
            let p = Paths {
                input: config.input.clone(),
                js: config.js.clone(),
                css: config.css.clone(),
            };
            Arc::new(p)
        };
        let slides_index = warp::path("slides").and(warp::path::end());
        warp::get()
            .and(slides_index)
            .and(warp::any().map(move || paths.clone()))
            .and(warp::any().map(move || renderer.clone()))
            .and_then(get_slides)
    };

    let ws = {
        let users = users.clone();
        let users = warp::any().map(move || users.clone());
        warp::path("ws")
            .and(warp::ws())
            .and(users)
            .map(|ws: warp::ws::Ws, users: Users| {
                let upgrade = move |socket| async {
                    if let Err(err) = handle_ws(socket, users).await {
                        error!("Failed to handle websocket, error: {}", err);
                    }
                };
                ws.on_upgrade(upgrade)
            })
    };

    let assets = {
        let assets_path: PathBuf = config
            .input
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .to_path_buf();
        warp::fs::dir(assets_path)
    };

    let routes = slides
        .or(ws)
        .or(assets)
        .with(warp::log("deck"))
        .recover(customize_error);

    // Configure server
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let server = warp::serve(routes).bind(addr);

    let mut slides_url = format!("{}/slides", addr);
    if config.watch {
        slides_url.push_str("?watch=true");
        info!("Watching {} for changes", config.input.to_string_lossy());
        let mut files = vec![config.input];
        if let Some(css) = config.css {
            files.push(css.clone());
        }
        if let Some(js) = config.js {
            files.push(js.clone());
        }
        let f = watch_files(files, users);
        tokio::task::spawn(f);
    }

    info!("Go to {} to see your slides", slides_url);

    server.await;

    Ok(())
}
