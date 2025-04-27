
///
/// [transport]
/// type = "tcp"
/// port = 8080
/// address = "127.0.0.1"
///

#[derive(Debug,serde::Deserialize)]
pub struct TransportConfig {
    pub r#type : String,
    pub port : u16,
    pub address : String
}