// use async_trait::async_trait;
// use log::{debug, error, info, warn};
// use reqwest::Client;
// use serde::{de::DeserializeOwned, Serialize};
// use std::collections::{HashMap, VecDeque};
// use std::sync::{Arc, Mutex};
// use std::time::{Duration, Instant};
// use tiny_http::{Method, Request, Response as HttpResponse, Server};
// use crate::MCPError;
// use crate::support::ControlBus;
// use crate::transport::common::{CloseCallback, ErrorCallback, MessageCallback, Transport};
// use crate::transport::IoProvider;

// /// Client connection information
// struct ClientConnection {
//     #[allow(dead_code)]
//     id: String,
//     last_poll: Instant,
// }

// /// Server-Sent Events (SSE) transport
// pub struct SSETransport {
//     notify: ControlBus,
//     provider: Box<dyn IoProvider + 'static>,
//     uri: String,
//     is_connected: bool,
//     is_server: bool,
//     on_close: Option<CloseCallback>,
//     on_error: Option<ErrorCallback>,
//     on_message: Option<MessageCallback>,
//     // HTTP client for making requests
//     client: Client,
//     // Queue for incoming messages
//     // message_queue: Arc<TokioMutex<VecDeque<String>>>,
//     // For server mode: active client connections
//     // active_clients: Arc<Mutex<HashMap<String, ClientConnection>>>,
//     // For server mode: client message queues
//     // client_messages: Arc<Mutex<HashMap<String, VecDeque<String>>>>,
//     // For client mode: client ID
//     // client_id: Arc<TokioMutex<Option<String>>>,
//     // Server instance
//     server: Option<Arc<Server>>,
// }

// impl SSETransport {
//     /// Create a new SSE transport
//     pub fn new(uri: &str) -> Self {
//         info!("Creating new SSE transport with URI: {}", uri);
//         Self {
//             notify: ControlBus::new(),
//             uri: uri.to_string(),
//             is_connected: false,
//             is_server: false,
//             on_close: None,
//             on_error: None,
//             on_message: None,
//             client: Client::new(),
//             server: None,
//         }
//     }

//     /// Create a new SSE transport in server mode
//     pub fn new_server(uri: &str) -> Self {
//         info!("Creating new SSE server transport with URI: {}", uri);
//         let mut transport = Self::new(uri);
//         transport.is_server = true;
//         transport
//     }
// }

// impl Transport for SSETransport {
//     async fn start(&mut self) -> Result<(), MCPError> {
//         if self.is_connected {
//             debug!("SSE transport already connected");
//             return Ok(());
//         }

//         info!("Starting SSE transport with URI: {}", self.uri);

//         if self.is_server {
//             // Parse the URI to get the host and port
//             let uri = self.uri.clone();
//             let uri_parts: Vec<&str> = uri.split("://").collect();
//             if uri_parts.len() != 2 {
//                 return Err(MCPError::Transport(format!("Invalid URI: {}", uri)));
//             }

//             let addr_parts: Vec<&str> = uri_parts[1].split(':').collect();
//             if addr_parts.len() != 2 {
//                 return Err(MCPError::Transport(format!("Invalid URI: {}", uri)));
//             }

//             let host = addr_parts[0];
//             let port: u16 = match addr_parts[1].parse() {
//                 Ok(p) => p,
//                 Err(_) => return Err(MCPError::Transport(format!("Invalid port in URI: {}", uri))),
//             };

//             let addr = format!("{}:{}", host, port);
//             info!("Starting SSE server on {}", addr);

//             // Create the HTTP server
//             let server = match Server::http(&addr) {
//                 Ok(s) => s,
//                 Err(e) => {
//                     return Err(MCPError::Transport(format!(
//                         "Failed to start HTTP server: {}",
//                         e
//                     )))
//                 }
//             };

//             let server_arc = Arc::new(server);
//             self.server = Some(Arc::clone(&server_arc));

//             // Start a task to handle incoming requests
//             let active_clients = Arc::clone(&self.active_clients);
//             let client_messages = Arc::clone(&self.client_messages);
//             let (sender, mut receiver) = mpsc::channel::<String>(32);

//             // Spawn a task to process incoming HTTP requests
//             let server_arc_clone = Arc::clone(&server_arc);
//             let stop_signal_clone = Arc::clone(&stop_signal);
//             let active_clients_clone = Arc::clone(&active_clients);
//             let client_messages_clone = Arc::clone(&client_messages);
//             let sender_clone = sender.clone();

//             tokio::spawn(async move {
//                 loop {
//                     // Check for stop signal with a small timeout
//                     let should_stop = tokio::time::timeout(
//                         Duration::from_millis(100),
//                         stop_signal_clone.notified(),
//                     )
//                         .await
//                         .is_ok();

//                     if should_stop {
//                         debug!("Server task received stop signal");
//                         break;
//                     }

//                     // Receive request (non-blocking)
//                     let server_for_recv = Arc::clone(&server_arc_clone);
//                     let request_result = tokio::task::spawn_blocking(move || {
//                         server_for_recv.recv_timeout(Duration::from_millis(50))
//                     })
//                         .await;

//                     // Process the request if we got one
//                     if let Ok(result) = request_result {
//                         if let Ok(Some(request)) = result {
//                             // Extract method and URL from the request
//                             let method = request.method().clone();
//                             let url = request.url().to_string();

//                             debug!("Server received {} request for {}", method, url);

//                             // Process request in a separate task to not block the main loop
//                             let sender_task = sender_clone.clone();
//                             let active_clients_task = Arc::clone(&active_clients_clone);
//                             let client_messages_task = Arc::clone(&client_messages_clone);

//                             tokio::spawn(async move {
//                                 process_request(
//                                     request,
//                                     &method,
//                                     &url,
//                                     &sender_task,
//                                     &active_clients_task,
//                                     &client_messages_task,
//                                 )
//                                     .await;
//                             });
//                         }
//                     }
//                 }
//                 debug!("Server HTTP handler task exited");
//             });

//             // Spawn a task to process messages received from clients
//             let message_queue_clone = Arc::clone(&message_queue);
//             let stop_signal_clone = Arc::clone(&stop_signal);
//             self.polling_task = Some(tokio::spawn(async move {
//                 loop {
//                     tokio::select! {
//                         Some(content) = receiver.recv() => {
//                             // Add the message to the server's message queue for processing
//                             let mut queue = message_queue_clone.lock().await;
//                             queue.push_back(content);
//                             debug!("Added message to server queue for processing");
//                         }
//                         _ = stop_signal_clone.notified() => {
//                             debug!("Server message processing task received stop signal");
//                             break;
//                         }
//                     }
//                 }
//                 debug!("Server message processing task exited");
//             }));
//         } else {
//             // For client mode - we'll use async polling
//             let uri = self.uri.clone();
//             let client = self.client.clone();
//             let client_id = Arc::clone(&self.client_id);
//             let message_queue_clone = Arc::clone(&message_queue);
//             let stop_signal_clone = Arc::clone(&stop_signal);

//             // Register with the server
//             debug!("Client registering with server at {}/register", uri);
//             match client.get(format!("{}/register", uri)).send().await {
//                 Ok(response) => {
//                     if response.status().is_success() {
//                         // Parse the client ID from the response
//                         match response.text().await {
//                             Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
//                                 Ok(json) => {
//                                     if let Some(id) =
//                                         json.get("client_id").and_then(|id| id.as_str())
//                                     {
//                                         debug!("Client registration successful with ID: {}", id);
//                                         let mut client_id_guard = client_id.lock().await;
//                                         *client_id_guard = Some(id.to_string());
//                                     } else {
//                                         warn!(
//                                             "Client registration response missing client_id field"
//                                         );
//                                     }
//                                 }
//                                 Err(e) => {
//                                     warn!("Failed to parse client registration response: {}", e);
//                                 }
//                             },
//                             Err(e) => {
//                                 warn!("Failed to read client registration response: {}", e);
//                             }
//                         }
//                     } else {
//                         warn!("Client registration failed: HTTP {}", response.status());
//                     }
//                 }
//                 Err(e) => {
//                     warn!("Client registration failed: {}", e);
//                 }
//             }

//             // Start a task to poll for messages
//             let client_id_clone = Arc::clone(&client_id);
//             let uri_clone = uri.clone();
//             let client_clone = client.clone();

//             // Simplify: Just use a polling task that adds messages to the queue
//             // The main thread will handle processing callbacks when messages are received
//             self.polling_task = Some(tokio::spawn(async move {
//                 loop {
//                     // Get the client ID
//                     let client_id_str = {
//                         let id_guard = client_id_clone.lock().await;
//                         id_guard.clone()
//                     };

//                     // Send a GET request to poll for messages
//                     let poll_uri = if let Some(id) = &client_id_str {
//                         format!("{}/poll?client_id={}", uri_clone, id)
//                     } else {
//                         format!("{}/poll", uri_clone)
//                     };
//                     debug!("Client polling for messages at {}", poll_uri);

//                     match client_clone.get(&poll_uri).send().await {
//                         Ok(response) => {
//                             if response.status().is_success() {
//                                 match response.text().await {
//                                     Ok(text) => {
//                                         if !text.is_empty() && text != "no_messages" {
//                                             debug!("Client received message from poll: {}", text);

//                                             // Try to parse as JSON to validate
//                                             match serde_json::from_str::<serde_json::Value>(&text) {
//                                                 Ok(_) => {
//                                                     // Add the message to the queue
//                                                     let mut queue =
//                                                         message_queue_clone.lock().await;
//                                                     queue.push_back(text.clone());
//                                                     debug!("Client added message to queue for processing");

//                                                     // The main thread will handle callbacks when messages are processed
//                                                 }
//                                                 Err(e) => {
//                                                     error!("Client received invalid JSON from server: {} - {}", e, text);
//                                                 }
//                                             }
//                                         } else {
//                                             debug!("Client: No new messages available");
//                                         }
//                                     }
//                                     Err(e) => {
//                                         error!("Client failed to read response text: {}", e);
//                                     }
//                                 }
//                             } else {
//                                 error!("Client poll request failed: HTTP {}", response.status());
//                             }
//                         }
//                         Err(e) => {
//                             error!("Client failed to poll for messages: {}", e);
//                             // Add a small delay before retrying to avoid hammering the server
//                             sleep(Duration::from_millis(1000)).await;
//                         }
//                     }

//                     // Check if we should stop polling
//                     if tokio::time::timeout(Duration::from_millis(0), stop_signal_clone.notified())
//                         .await
//                         .is_ok()
//                     {
//                         debug!("Client polling task received stop signal");
//                         break;
//                     }

//                     // Wait before polling again
//                     sleep(Duration::from_millis(500)).await;
//                 }
//                 debug!("Client polling task exited");
//             }));
//         }

//         self.is_connected = true;
//         info!("SSE transport started successfully");
//         Ok(())
//     }

//     async fn send<T: Serialize + Send + Sync>(&mut self, message: &T) -> Result<(), MCPError> {
//         if !self.is_connected {
//             return Err(MCPError::Transport(
//                 "SSE transport not connected".to_string(),
//             ));
//         }

//         // Serialize the message to JSON
//         let serialized_message = match serde_json::to_string(message) {
//             Ok(json) => json,
//             Err(e) => {
//                 let error_msg = format!("Failed to serialize message: {}", e);
//                 error!("{}", error_msg);
//                 return Err(MCPError::Serialization(e));
//             }
//         };
//         debug!("Sending message: {}", serialized_message);

//         if self.is_server {
//             // Server mode - add the message to the client message queue
//             debug!(
//                 "Server adding message to client queue: {}",
//                 serialized_message
//             );

//             // In server mode, we need to add the message to the client message queue
//             // This is a separate queue from the server's message queue
//             if let Ok(clients) = self.active_clients.lock() {
//                 // Add the message to all connected clients' queues
//                 for client_id in clients.keys() {
//                     if let Ok(mut client_messages) = self.client_messages.lock() {
//                         client_messages
//                             .entry(client_id.clone())
//                             .or_insert_with(VecDeque::new)
//                             .push_back(serialized_message.clone());
//                         debug!("Added message to client {}'s queue", client_id);
//                     }
//                 }
//                 debug!("Server successfully added message to client queues");
//                 Ok(())
//             } else {
//                 error!("Failed to lock active clients");
//                 Err(MCPError::Transport(
//                     "Failed to lock active clients".to_string(),
//                 ))
//             }
//         } else {
//             // Client mode - send a POST request to the server
//             debug!("Client sending message to server: {}", serialized_message);

//             match self
//                 .client
//                 .post(&self.uri)
//                 .body(serialized_message.clone())
//                 .header(reqwest::header::CONTENT_TYPE, "application/json")
//                 .send()
//                 .await
//             {
//                 Ok(response) => {
//                     if response.status().is_success() {
//                         debug!("Client successfully sent message to server");
//                         Ok(())
//                     } else {
//                         let error_msg = format!(
//                             "Failed to send message to server: HTTP {}",
//                             response.status()
//                         );
//                         error!("{}", error_msg);
//                         Err(MCPError::Transport(error_msg))
//                     }
//                 }
//                 Err(e) => {
//                     let error_msg = format!("Failed to send message to server: {}", e);
//                     error!("{}", error_msg);
//                     Err(MCPError::Transport(error_msg))
//                 }
//             }
//         }
//     }

//     async fn receive<T: DeserializeOwned + Send + Sync>(&mut self) -> Result<T, MCPError> {
//         if !self.is_connected {
//             return Err(MCPError::Transport(
//                 "SSE transport not connected".to_string(),
//             ));
//         }

//         // Use a timeout of 10 seconds
//         let timeout = Duration::from_secs(10);
//         let start = Instant::now();

//         // Try to get a message from the queue with timeout
//         let message = loop {
//             // Try to get a message from the queue
//             let queue_msg = {
//                 let mut queue = self.message_queue.lock().await;
//                 queue.pop_front()
//             };

//             if let Some(message) = queue_msg {
//                 debug!("Received message: {}", message);
//                 break message;
//             }

//             // Check if we've exceeded the timeout
//             if start.elapsed() >= timeout {
//                 debug!("Receive timeout after {:?}", timeout);
//                 return Err(MCPError::Transport(
//                     "Timeout waiting for message".to_string(),
//                 ));
//             }

//             // Sleep for a short time before checking again
//             sleep(Duration::from_millis(100)).await;
//         };

//         // Parse the message
//         match serde_json::from_str::<T>(&message) {
//             Ok(parsed) => {
//                 debug!("Successfully parsed message");
//                 Ok(parsed)
//             }
//             Err(e) => {
//                 let error_msg = format!(
//                     "Failed to deserialize message: {} - Content: {}",
//                     e, message
//                 );
//                 error!("{}", error_msg);
//                 Err(MCPError::Serialization(e))
//             }
//         }
//     }

//     async fn close(&mut self) -> Result<(), MCPError> {
//         if !self.is_connected {
//             debug!("SSE transport already closed");
//             return Ok(());
//         }

//         info!("Closing SSE transport for URI: {}", self.uri);

//         // Set the connection flag
//         self.is_connected = false;

//         // Signal the polling task to stop
//         self.stop_signal.notify_waiters();

//         // If we're a server, wait a short time to allow clients to receive final responses
//         if self.is_server {
//             debug!("Server waiting for clients to receive final responses");
//             // Wait a short time to allow clients to receive final responses
//             sleep(Duration::from_millis(1000)).await;
//         }

//         // Join the polling task if it exists
//         if let Some(task) = self.polling_task.take() {
//             match task.abort() {
//                 _ => debug!("Aborted polling task"),
//             }
//         }

//         // Call the close callback if set
//         if let Some(callback) = &self.on_close {
//             callback();
//         }

//         info!("SSE transport closed successfully");
//         Ok(())
//     }

//     fn set_on_close(&mut self, callback: Option<CloseCallback>) {
//         debug!("Setting on_close callback for SSE transport");
//         self.on_close = callback;
//     }

//     fn set_on_error(&mut self, callback: Option<ErrorCallback>) {
//         debug!("Setting on_error callback for SSE transport");
//         self.on_error = callback;
//     }

//     fn set_on_message<F>(&mut self, callback: Option<F>)
//     where
//         F: Fn(&str) + Send + Sync + 'static,
//     {
//         debug!("Setting on_message callback for SSE transport");
//         self.on_message = callback.map(|f| Box::new(f) as Box<dyn Fn(&str) + Send + Sync>);
//     }
// }

// // Helper function to process HTTP requests
// async fn process_request(
//     mut request: Request,
//     method: &Method,
//     url: &str,
//     sender: &mpsc::Sender<String>,
//     active_clients: &Arc<Mutex<HashMap<String, ClientConnection>>>,
//     client_messages: &Arc<Mutex<HashMap<String, VecDeque<String>>>>,
// ) {
//     match (method, url) {
//         (Method::Post, "/") => {
//             // Handle POST request (client sending a message to server)
//             let mut content = String::new();
//             if let Err(e) = request.as_reader().read_to_string(&mut content) {
//                 error!("Error reading request body: {}", e);
//                 let _ = request.respond(
//                     HttpResponse::from_string("Error reading request").with_status_code(400),
//                 );
//                 return;
//             }

//             debug!("Server received POST request body: {}", content);

//             // Send the message to be processed
//             if let Err(e) = sender.send(content).await {
//                 error!("Failed to send message to processing task: {}", e);
//             }

//             // Send a success response
//             let _ = request.respond(HttpResponse::from_string("OK").with_status_code(200));
//         }
//         (Method::Get, path) if path.starts_with("/poll") => {
//             // Handle polling request from client
//             debug!("Server received polling request: {}", path);

//             // Extract client ID from query parameters
//             let client_id = path.split('?').nth(1).and_then(|query| {
//                 query.split('&').find_map(|pair| {
//                     let mut parts = pair.split('=');
//                     if let Some(key) = parts.next() {
//                         if key == "client_id" {
//                             parts.next().map(|value| value.to_string())
//                         } else {
//                             None
//                         }
//                     } else {
//                         None
//                     }
//                 })
//             });

//             if let Some(client_id) = client_id {
//                 // Check if there are any messages in the client-specific queue
//                 let message = if let Ok(mut client_msgs) = client_messages.lock() {
//                     client_msgs
//                         .entry(client_id.clone())
//                         .or_insert_with(VecDeque::new)
//                         .pop_front()
//                 } else {
//                     None
//                 };

//                 // Send the message or a no-message response
//                 if let Some(msg) = message {
//                     debug!("Server sending message to client {}: {}", client_id, msg);
//                     let response = HttpResponse::from_string(msg)
//                         .with_status_code(200)
//                         .with_header(tiny_http::Header {
//                             field: "Content-Type".parse().unwrap(),
//                             value: "application/json".parse().unwrap(),
//                         });

//                     if let Err(e) = request.respond(response) {
//                         error!("Failed to send response to client: {}", e);
//                     } else {
//                         debug!("Server successfully sent response to client");
//                     }
//                 } else {
//                     // No messages available
//                     debug!(
//                         "Server sending no_messages response to client {}",
//                         client_id
//                     );
//                     let response = HttpResponse::from_string("no_messages").with_status_code(200);

//                     if let Err(e) = request.respond(response) {
//                         error!("Failed to send no_messages response: {}", e);
//                     }
//                 }

//                 // Update the client's last poll time
//                 if let Ok(mut clients) = active_clients.lock() {
//                     if let Some(client) = clients.get_mut(&client_id) {
//                         client.last_poll = Instant::now();
//                     }
//                 }
//             } else {
//                 // No client ID provided
//                 debug!("Client poll request missing client_id parameter");
//                 let response =
//                     HttpResponse::from_string("Missing client_id parameter").with_status_code(400);
//                 let _ = request.respond(response);
//             }
//         }
//         (Method::Get, "/register") => {
//             // Handle client registration
//             debug!("Server received client registration request");

//             // Track the client connection
//             let client_id = format!(
//                 "client-{}",
//                 std::time::SystemTime::now()
//                     .duration_since(std::time::UNIX_EPOCH)
//                     .unwrap_or_default()
//                     .as_millis()
//             );

//             if let Ok(mut clients) = active_clients.lock() {
//                 clients.insert(
//                     client_id.clone(),
//                     ClientConnection {
//                         id: client_id.clone(),
//                         last_poll: Instant::now(),
//                     },
//                 );
//                 debug!("Client registered: {}", client_id);
//                 debug!("Total connected clients: {}", clients.len());
//             }

//             // Initialize the client's message queue
//             if let Ok(mut client_msgs) = client_messages.lock() {
//                 client_msgs
//                     .entry(client_id.clone())
//                     .or_insert_with(VecDeque::new);
//                 debug!("Initialized message queue for client {}", client_id);
//             }

//             // Send a success response
//             let response =
//                 HttpResponse::from_string(format!("{{\"client_id\":\"{}\"}}", client_id))
//                     .with_status_code(200)
//                     .with_header(tiny_http::Header {
//                         field: "Content-Type".parse().unwrap(),
//                         value: "application/json".parse().unwrap(),
//                     });

//             if let Err(e) = request.respond(response) {
//                 error!("Failed to send registration response: {}", e);
//             } else {
//                 debug!("Server successfully registered client");
//             }
//         }
//         _ => {
//             // Unsupported method or path
//             error!("Unsupported request: {} {}", method, url);
//             let _ = request.respond(
//                 HttpResponse::from_string("Method or path not allowed").with_status_code(405),
//             );
//         }
//     }
// }
