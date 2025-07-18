use crate::routes::{health_check, subscribe, confirm};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;
use crate::email_client::EmailClient;
use crate::configuration::Settings;
use sqlx::postgres::PgPoolOptions;
use crate::configuration::DatabaseSettings;

pub struct Application{
    port :u16,
    server:Server
}
impl Application{
    pub async fn build(configuration:Settings) ->Result<Self, std::io::Error>{
    let connection_pool = PgPoolOptions::new()
    .connect_lazy_with(configuration.database.with_db());

    let sender_email= configuration.email_client.sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(configuration.email_client.base_url, sender_email,
        configuration.email_client.authorization_token,
        timeout);
    let address = format!("{}:{}", configuration.application.host, configuration.application.port);
    let listener = TcpListener::bind(address)?;
    let port = listener.local_addr().unwrap().port();
    let server = run(listener, connection_pool,email_client, configuration.application.base_url)?;
    Ok(Self {port, server})
}
    pub fn port(&self)->u16{
        self.port
    }

    pub async fn run_until_stopped(self) ->Result<(),std::io::Error>{
        self.server.await
    }
}
pub struct ApplicationBaseUrl(pub String);
pub fn run(
    listener:TcpListener,
    connection:PgPool,
    email_client:EmailClient,
    base_url:String,
) ->Result<Server, std::io::Error>{
    let connection = web::Data:: new(connection);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move ||{
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .app_data(connection.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
pub fn get_connection_pool(
    configuration:&DatabaseSettings)->PgPool{
        PgPoolOptions::new().connect_lazy_with(configuration.with_db())
    }