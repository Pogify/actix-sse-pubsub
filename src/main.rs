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
use std::sync::RwLock;
use std::task::{Context, Poll};
use std::time::Duration;

use actix_cors::Cors;
use actix_web::web::{Bytes, Data, Path};
use actix_web::{get, post, web, App, Error, HttpResponse, HttpServer, Responder};
use clap::{crate_version, clap_app};
use futures::Stream;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::time::{interval_at, Instant};


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let matches = clap_app!(myapp =>
        (version: crate_version!())
        (author: "Ronak B.")
        (about: "A sse-based pubsub with different channels")
        (@arg HOST: --host +takes_value "Address to host on")
        (@arg PORT: --port +takes_value "Port to host on")
        (@arg KEY: --key +takes_value "SSL Private Key file")
        (@arg CERT: --cert +takes_value "Certificate file")
    ).get_matches();

    let data = BroadcasterMap::create();


    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::new().finish())
            .app_data(data.clone())
            .service(index)
            .service(new_client)
            .service(broadcast)
    });
    let host = format!("{}:{}",
            matches.value_of("HOST").unwrap_or("127.0.0.1"),
            matches.value_of("PORT").unwrap_or("8080"));

    let server = match (matches.value_of("KEY"), matches.value_of("CERT")) {
        (Some(keyfile), Some(certfile)) => {
            let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
            builder
                .set_private_key_file(keyfile, SslFiletype::PEM)
                .expect("Invalid private key provided");
            builder.set_certificate_chain_file(certfile).expect("Invalid certificate provided");
            server.bind_openssl(host, builder)
        },
        _ => server.bind(host)
    };

    server
        .expect("Failed to bind")
        .maxconn(500000)
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
async fn new_client(channel: Path<String>, broadcasters: Data<RwLock<BroadcasterMap>>) -> impl Responder {

    let rx = broadcasters.write().unwrap().new_client(&channel.into_inner());

    HttpResponse::Ok()
        .header("content-type", "text/event-stream")
        .no_chunking()
        .streaming(rx)
}

#[post("/broadcast")]
async fn broadcast(
    data: web::Json<SpotifyData>,
    broadcasters: Data<RwLock<BroadcasterMap>>,
) -> impl Responder {

    let channel = &data.channel;
    let msg = &data.msg;

    broadcasters.read().unwrap().broadcast(&channel, &msg);
    HttpResponse::Ok()
        .body("idk if it sent or not")
}

#[derive(Serialize, Deserialize)]
struct SpotifyData {
    channel: String,
    msg: String,
}

struct Broadcaster {
    senders: Vec<Sender<Bytes>>,
    num_clients: u32,
}

impl Broadcaster {
    fn new() -> Broadcaster {
        Broadcaster {
            senders: Vec::new(),
            num_clients: 1,
        }
    }

    // fn send(&msg) {
    //     
    // }
}

struct BroadcasterMap {
    broadcasters: HashMap<String, Sender<Bytes>>,
    channel_num: usize,
}

impl BroadcasterMap {
    fn new() -> Self {
        BroadcasterMap {
            broadcasters: HashMap::<String, Sender<Bytes>>::new(),
            channel_num: 500000
        }
    }

    fn create() -> Data<RwLock<Self>> {
        Data::new(RwLock::new(BroadcasterMap::new()))
    }

    fn spawn_ping(tx: Sender<Bytes>) {
        actix_rt::spawn(async move {
            let mut task = interval_at(Instant::now(), Duration::from_secs(10));
            while let _ = task.tick().await {
                tx.send(Bytes::from("data: ping\n\n"));
            }
        });
    }

    fn new_client(&mut self, room: &str) -> Client {
        let s = (&room).to_string();
        match self.broadcasters.get_mut(&s) {
            Some(broadcaster) => Client(broadcaster.subscribe()),
            None => {
                let (tx, rx) = channel(self.channel_num);
                // BroadcasterMap::spawn_ping(tx.clone());
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
