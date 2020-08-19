/* 
 * This has been built upon the previous work as mentioned in the readme
 *
 * File is only using the Client struct from previous work which implements stream
 *
 * I have changed it so that it can support multiple channels via broadcast
 *
 */


use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};

use actix_cors::Cors;
use actix_web::web::{Bytes, Data, Path};
use actix_web::{get, post, web, App, Error, HttpResponse, HttpServer, Responder};
use clap::{crate_version, clap_app};
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{channel, Receiver, Sender};


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let matches = clap_app!(myapp =>
        (version: crate_version!())
        (author: "Ronak B.")
        (about: "A sse-based pubsub with different channels")
        (@arg HOST: --host +takes_value "Address to host on")
        (@arg PORT: --port +takes_value "Port to host on")
    ).get_matches();

    let data = BroadcasterMap::create();


    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::new()
                    .allowed_origin("*")
                    .allowed_methods(vec!["GET", "POST"])
                    .finish())
            .app_data(data.clone())
            .service(index)
            .service(new_client)
            .service(broadcast)
    })
    .bind(format!("{}:{}",
            matches.value_of("HOST").unwrap_or("127.0.0.1"),
            matches.value_of("PORT").unwrap_or("8080")))?
    .run()
    .await
}

#[get("/")]
async fn index() -> impl Responder {
    let content = include_str!("index.html");

    HttpResponse::Ok()
        .header("content-type", "text/html")
        .body(content)
}

#[get("/events/{channel}")]
async fn new_client(channel: Path<String>, broadcasters: Data<Mutex<BroadcasterMap>>) -> impl Responder {

    let rx = broadcasters.lock().unwrap().new_client(&channel.into_inner());

    HttpResponse::Ok()
        .header("content-type", "text/event-stream")
        .no_chunking()
        .streaming(rx)
}

#[post("/broadcast")]
async fn broadcast(
    data: web::Json<SpotifyData>,
    broadcasters: Data<Mutex<BroadcasterMap>>,
) -> impl Responder {

    let channel = &data.channel;
    let msg = &data.msg;

    broadcasters.lock().unwrap().broadcast(&channel, &msg);
    HttpResponse::Ok()
        .body("idk if it sent or not")
}

#[derive(Serialize, Deserialize)]
struct SpotifyData {
    channel: String,
    msg: String,
}

struct BroadcasterMap {
    broadcasters: HashMap<String, Sender<Bytes>>,
    channel_num: usize,
}

impl BroadcasterMap {
    fn new() -> Self {
        BroadcasterMap {
            broadcasters: HashMap::<String, Sender<Bytes>>::new(),
            channel_num: 50000
        }
    }

    fn create() -> Data<Mutex<Self>> {
        Data::new(Mutex::new(BroadcasterMap::new()))
    }

    fn new_client(&mut self, room: &str) -> Client {
        let s = (&room).to_string();
        match self.broadcasters.get_mut(&s) {
            Some(broadcaster) => Client(broadcaster.subscribe()),
            None => {
                let (tx, rx) = channel(self.channel_num);
                self.broadcasters.insert(s.to_string(), tx);
                Client(rx)
            }
        }
    }

    fn broadcast(&self, channel: &str, msg: &str) {
        let s = (&channel).to_string();
        let encoded_msg = Bytes::from(["data: ", msg, "\n\n"].concat());
        // TODO return http responses with according messages
        match self.broadcasters.get(&s) {
            Some(broadcaster) => {
                match broadcaster.send(encoded_msg) {
                    Ok(_) => (),
                    Err(_e) => (),
                }
            },
            None => (),
        }
    }
}

// wrap Receiver in own type, with correct error type
struct Client(Receiver<Bytes>);

impl Stream for Client {
    type Item = Result<Bytes, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // match needed for ``Poll::Ready(Some(Err(v)))`` case
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Ready(Some(Ok(v))) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(Some(Err(_))) => Poll::Ready(None),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
