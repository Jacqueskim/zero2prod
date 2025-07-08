use zero2prod::startup::run;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use secrecy::ExposeSecret;
#[tokio::main]
async fn main() ->Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    
   
    let configuration = get_configuration().expect("Failed to read configuratrion.");
    let connection_pool = PgPoolOptions::new()
    .connect_lazy_with(configuration.database.with_db());
        // .expect("Failed to connect to Postgres.");
    let address = format!("{}:{}", configuration.application.host, configuration.application.port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await?;
    
    Ok(())
}

