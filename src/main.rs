mod argp;
mod persistence;
mod pop3;
mod tcpstream;

use pop3::{POP3Client};

#[derive(Debug)]
pub struct AppConfig {
    pub username: String,
    pub password: String,
    pub db_name: String,
}

fn main() {
    let app_config = argp::arg_parse();
    let mut client = POP3Client::new(app_config).unwrap();
    client.fetch_emails();
}
