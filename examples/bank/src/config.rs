use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub rust_log: Option<String>,
    pub database_url: String,
    pub redpanda_host: String,
}
