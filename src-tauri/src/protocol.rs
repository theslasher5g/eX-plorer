//! URI-Schema `shellimg`: liefert Icons und Thumbnails als PNG.
//!
//! `http://shellimg.localhost/?p=<Pfad>&s=<Größe>`. Der Pfad wird wie jeder
//! andere Pfad vom Frontend validiert. Die Arbeit läuft im Pool, nie im
//! UI-Thread; dort, wo nichts erzeugt werden darf (Cloud, Netzwerk), gibt es
//! 404 und das Frontend bleibt bei seinem gezeichneten Symbol.

use std::borrow::Cow;
use std::sync::OnceLock;

use explorer_core::path::validate_path;
use explorer_core::CoreError;
use tauri::http::{header, Request, Response};
use tauri::UriSchemeResponder;

use crate::shell::images;
use crate::workers::{Pool, Task};

pub const SCHEME: &str = "shellimg";
const THREADS: usize = 3;
const MAX_QUEUE: usize = 512;

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| Pool::new(THREADS, MAX_QUEUE))
}

fn reply(status: u16, body: Vec<u8>) -> Response<Cow<'static, [u8]>> {
    let mut builder = Response::builder().status(status);
    if status == 200 {
        builder = builder
            .header(header::CONTENT_TYPE, "image/png")
            // Die URL enthält die Änderungszeit, daher darf der Browser lange zwischenspeichern.
            .header(header::CACHE_CONTROL, "private, max-age=3600");
    }
    builder
        .body(Cow::Owned(body))
        .unwrap_or_else(|_| Response::new(Cow::Borrowed(&[][..])))
}

pub fn handle(request: &Request<Vec<u8>>, responder: UriSchemeResponder) {
    let mut path = None;
    let mut size = 32u32;
    if let Some(query) = request.uri().query() {
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            match key.as_ref() {
                "p" => path = Some(value.into_owned()),
                "s" => size = value.parse().unwrap_or(32),
                _ => {}
            }
        }
    }
    let Some(valid) = path.as_deref().and_then(|p| validate_path(p).ok()) else {
        return responder.respond(reply(400, Vec::new()));
    };

    let responder = std::sync::Arc::new(std::sync::Mutex::new(Some(responder)));
    let on_reject = std::sync::Arc::clone(&responder);
    let respond = move |response: Response<Cow<'static, [u8]>>| {
        let taken = responder.lock().ok().and_then(|mut guard| guard.take());
        if let Some(responder) = taken {
            responder.respond(response);
        }
    };
    let respond_reject = {
        let respond_slot = on_reject;
        move || {
            let taken = respond_slot.lock().ok().and_then(|mut guard| guard.take());
            if let Some(responder) = taken {
                responder.respond(reply(503, Vec::new()));
            }
        }
    };

    pool().submit(Task {
        run: Box::new(move || {
            let response = match images::render_png(&valid, size) {
                Ok(png) => reply(200, png),
                Err(CoreError::NotFound) => reply(404, Vec::new()),
                Err(CoreError::Unsupported | CoreError::TooLarge) => reply(404, Vec::new()),
                Err(_) => reply(500, Vec::new()),
            };
            respond(response);
        }),
        reject: Box::new(respond_reject),
    });
}
