use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use super::playlist::PlayList;

pub struct MediaServer {}
impl MediaServer {
    pub async fn start(playlist: Arc<Mutex<PlayList>>) {
        let address = "0.0.0.0:1337".parse().unwrap();
        let make_service = make_service_fn(move |_| {
            let playlist = playlist.clone();
            async { Ok::<_, hyper::Error>(service_fn(move |request| handle_request(request, playlist.clone()))) }
        });
        let server = Server::bind(&address).serve(make_service);
        println!("media server on http://{}", address);
        if let Err(e) = server.await {
            println!("media server error: {}", e);
        }
    }
}

async fn handle_request(req: Request<Body>, playlist: Arc<Mutex<PlayList>>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/status") => {
            let playlist = playlist.lock().unwrap();
            let json = format!("{{\"live\": {}}}", playlist.live);
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("content-type", "application/json")
                .body(json.into())
                .unwrap())
        }
        (&Method::GET, "/video.m3u8") => {
            let playlist = playlist.lock().unwrap();
            if playlist.live {
                let m3u8 = playlist.m3u8.clone();
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Cache-Control", "no-cache, no-store, must-revalidate")
                    .header("Pragma", "no-cache")
                    .header("Expires", "0")
                    .header("content-type", "application/vnd.apple.mpegurl")
                    .body(m3u8.into())
                    .unwrap());
            }
            Ok(file_not_found())
        }
        (&Method::GET, _) => file_response(&format!("./video/{}", req.uri().path())).await,
        _ => Ok(file_not_found()),
    }
}

fn file_not_found() -> Response<Body> {
    Response::builder().status(StatusCode::NOT_FOUND).body("404 NOT FOUND".into()).unwrap()
}

async fn file_response(path: &str) -> Result<Response<Body>, hyper::Error> {
    if let Ok(file) = File::open(path).await {
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = Body::wrap_stream(stream);
        return Ok(Response::builder().header("Access-Control-Allow-Origin", "*").body(body).unwrap());
    }
    Ok(file_not_found())
}
