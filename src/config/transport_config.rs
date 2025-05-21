
///
/// [transport]
/// type = "tcp"
/// port = 8080
/// address = "127.0.0.1"
///

#[derive(Debug,serde::Deserialize)]
pub struct HttpTransportConfig {
    pub port : u16,
    pub ip_address : String,
    pub enable_tls: bool,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
}