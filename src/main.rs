use hyper::{Body, Request, Response, Server, Method};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Deserialize, Serialize, Debug)]
struct Data {
    name: String,
    age: u32,
}

async fn handle_request(req: Request<Body>, file_path: Arc<Mutex<String>>) -> Result<Response<Body>, Infallible> {
    match req.method() {
        &Method::POST => {
            let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let data: Data = match serde_json::from_slice(&whole_body) {
                Ok(data) => data,
                Err(_) => return Ok(Response::builder().status(400).body(Body::from("Invalid JSON")).unwrap()),
            };
            
            // Writing the data to a JSON file
            let json_data = serde_json::to_string_pretty(&data).unwrap();
            let file_path = file_path.lock().await;
            let mut file = File::create(&*file_path).await.unwrap();
            file.write_all(json_data.as_bytes()).await.unwrap();
            
            Ok(Response::new(Body::from("Data successfully written to JSON file")))
        }
        _ => {
            Ok(Response::builder().status(405).body(Body::from("Method Not Allowed")).unwrap())
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();
    let file_path = Arc::new(Mutex::new("data.json".to_string()));

    let make_svc = make_service_fn(move |_conn| {
        let file_path = file_path.clone();
        async move { 
            Ok::<_, Infallible>(service_fn(move |req| handle_request(req, file_path.clone()))) 
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Server running on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}
