use hyper::{Body, Request, Response, Server, Method};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::str;

// Define a struct to hold data, with fields for name and age
#[derive(Deserialize, Serialize, Debug)]
struct Data {
    name: String,
    age: u32,
}

// Define an async function to handle incoming requests
async fn handle_request(req: Request<Body>, file_path: Arc<Mutex<String>>) -> Result<Response<Body>, Infallible> {
    match req.method() {
        &Method::POST => {
            // Read the entire request body into a byte array
            let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
            
            // Convert bytes to string and log it
            let body_str = str::from_utf8(&whole_body).unwrap_or("<Invalid UTF-8>");
            println!("Received body: {}", body_str);
            
            // Deserialize the byte array into a Vec<Data> struct
            let incoming_data: Vec<Data> = match serde_json::from_slice(&whole_body) {
                Ok(data) => data,
                Err(e) => {
                    // Log the error and return a 400 error response
                    eprintln!("Deserialization error: {}", e);
                    return Ok(Response::builder().status(400).body(Body::from("Invalid JSON")).unwrap());
                }
            };
            
            // Lock the file path mutex to ensure thread safety
            let file_path = file_path.lock().await;
            
            // Open the JSON file and read its contents
            let mut file = OpenOptions::new().read(true).write(true).create(true).open(&*file_path).await.unwrap();
            let mut file_content = String::new();
            file.read_to_string(&mut file_content).await.unwrap();

            // Deserialize the existing data into a Vec<Data>, or create an empty Vec if the file is empty
            let mut existing_data: Vec<Data> = if file_content.is_empty() {
                Vec::new()
            } else {
                match serde_json::from_str(&file_content) {
                    Ok(data) => data,
                    Err(e) => {
                        // Log the error and start with an empty Vec
                        eprintln!("Existing data deserialization error: {}", e);
                        Vec::new()
                    }
                }
            };

            // Add the new data to the existing data
            existing_data.extend(incoming_data);

            // Serialize the updated data back into a JSON string
            let json_data = serde_json::to_string_pretty(&existing_data).unwrap();

            // Truncate the file and write the updated JSON data to the file
            let mut file = File::create(&*file_path).await.unwrap();
            file.write_all(json_data.as_bytes()).await.unwrap();
            
            // Return a success response
            Ok(Response::new(Body::from("Data successfully appended to JSON file")))
        }
        // If the request method is not POST, return a 405 error response
        _ => {
            Ok(Response::builder().status(405).body(Body::from("Method Not Allowed")).unwrap())
        }
    }
}

// Define the main function, marked with the tokio::main attribute
#[tokio::main]
async fn main() {
    // Define the address to bind the server to
    let addr = ([127, 0, 0, 1], 3000).into();
    // Create a mutex to hold the file path, wrapped in an Arc for thread safety
    let file_path = Arc::new(Mutex::new("data.json".to_string()));

    // Define a function to create a new service for each incoming connection
    let make_svc = make_service_fn(move |_conn| {
        // Clone the file path mutex for each connection
        let file_path = file_path.clone();
        // Define an async function to handle the request
        async move { 
            // Return a new service function that calls handle_request
            Ok::<_, Infallible>(service_fn(move |req| handle_request(req, file_path.clone()))) 
        }
    });

    // Create a new server that binds to the specified address and uses the make_svc function
    let server = Server::bind(&addr).serve(make_svc);

    // Print a message indicating that the server is running
    println!("Server running on http://{}", addr);

    // Run the server and handle any errors that occur
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}
