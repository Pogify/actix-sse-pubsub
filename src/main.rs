/* 
 * This has been built upon the previous work as mentioned in the readme
 *
 * I have changed it so that it can support multiple channels
 *
 */


use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::Duration;

use actix_cors::Cors;
use actix_web::web::{Bytes, Data, Path};
use actix_web::{get, post, web, App, Error, HttpResponse, HttpServer, Responder};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{interval_at, Instant};


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

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
    .bind("127.0.0.1:8080")?
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

    match broadcasters.lock().unwrap().get_broadcaster(&channel) {
        Some(broadcaster) => {
            broadcaster.lock().unwrap().send(&msg);

            HttpResponse::Ok()
                .body("msg sent")
        },
        None => {
            HttpResponse::NotFound()
                .body("you have no clients yet")
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SpotifyData {
    channel: String,
    msg: String
}

struct BroadcasterMap {
    broadcasters: HashMap<String, Data<Mutex<Broadcaster>>>,
}

impl BroadcasterMap {
    fn new() -> Self {
        BroadcasterMap {
            broadcasters: HashMap::<String, Data<Mutex<Broadcaster>>>::new(),
        }
    }

    fn create() -> Data<Mutex<Self>> {
        Data::new(Mutex::new(BroadcasterMap::new()))
    }

    fn new_client(&mut self, channel: &str) -> Client {
        let s = (&channel).to_string();
        match self.broadcasters.get_mut(&s) {
            Some(broadcaster) => broadcaster.lock().unwrap().new_client(),
            None => {
                self.broadcasters.insert(s.to_string(), Broadcaster::create());
                self.broadcasters.get_mut(&s).unwrap()
                    .lock().unwrap()
                    .new_client()
            }
        }
    }

    fn get_broadcaster(&self, channel: &str) -> Option<&Data<Mutex<Broadcaster>>> {
        self.broadcasters.get(&(&channel).to_string())
    }
}


struct Broadcaster {
    clients: Vec<Sender<Bytes>>,
}

impl Broadcaster {
    fn create() -> Data<Mutex<Self>> {
        // Data ≃ Arc (thread-safe smart pointer)
        let me = Data::new(Mutex::new(Broadcaster::new()));

        // ping clients every 10 seconds to see if they are alive
        Broadcaster::spawn_ping(me.clone());

        me
    }

    fn new() -> Self {
        Broadcaster {
            clients: Vec::<Sender<Bytes>>::new(),
        }
    }

    fn spawn_ping(me: Data<Mutex<Self>>) {
        actix_rt::spawn(async move {
            let mut task = interval_at(Instant::now(), Duration::from_secs(10));
            while let Some(_) = task.next().await {
                me.lock().unwrap().remove_stale_clients();
            }
        })
    }

    fn remove_stale_clients(&mut self) {
        let mut ok_clients = Vec::new();
        for client in self.clients.iter() {
            let result = client.clone().try_send(Bytes::from("data: ping\n\n"));

            if let Ok(()) = result {
                ok_clients.push(client.clone());
            }
        }
        self.clients = ok_clients;
    }

    fn new_client(&mut self) -> Client {
        let (tx, rx) = channel(100);

        tx.clone()
            .try_send(Bytes::from("data: connected\n\n"))
            .unwrap();

        self.clients.push(tx);
        Client(rx)
    }

    fn send(&self, msg: &str) {
        let msg = Bytes::from(["data: ", msg, "\n\n"].concat());

        for client in self.clients.iter() {
            client.clone().try_send(msg.clone()).unwrap_or(());
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
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
