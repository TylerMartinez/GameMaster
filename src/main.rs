use std::net::SocketAddr;
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

use serde_json::json;

use dotenv::dotenv;

use http_body_util::Full;
use hyper::service::Service;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use hyper::{Method, StatusCode};
use http_body_util::BodyExt;

use serenity::http::client::Http;
use serenity::http::client::HttpBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    dotenv().ok();

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io,
                    DiscordService {
                        client: Arc::new(
                            HttpBuilder::new("")
                            .build()
                        )
                    }
                )
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

struct DiscordService {
    client: Arc<Http>,
}

impl Service<Request<IncomingBody>> for DiscordService {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        fn mk_response(body: String, status: StatusCode) -> Result<Response<Full<Bytes>>, hyper::Error> {
            Ok(Response::builder()
                .status(status)
                .body(Full::new(Bytes::from(body)))
                .unwrap()
            )
        }

        let discord = self.client.clone();

        let res = match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => mk_response(
                "Try POSTing data to /echo".to_string(),
                StatusCode::OK
            ),
    
            (&Method::POST, "/echo") => return Box::pin( async move { 
                let message = json!({
                    "content": "hello"
                });

                discord.send_message(
                    588518200715640833, 
                    &message
                ).await;

                Ok(Response::builder().body(Full::new(req.collect().await?.to_bytes())).unwrap())
            }),
    
            // Return 404 Not Found for other routes.
            _ => {
                mk_response("".to_string(), StatusCode::NOT_FOUND)
            }
        };
    

        Box::pin(async { res })
    }
}
