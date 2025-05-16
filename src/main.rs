use zero2prod::run;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use sqlx::{Connection, PgConnection};

#[tokio::main]
async fn main() ->Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuratrion.");
    let connection = PgConnection::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener)?.await
    
    
}
