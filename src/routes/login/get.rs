use actix_web::{web, HttpResponse, http::header::ContentType};
use crate::startup::HmacSecret;
use hmac::{Hmac, Mac, NewMac};
use secrecy::ExposeSecret;
use actix_web::cookie::{Cookie, time::Duration};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;
#[derive(serde::Deserialize)]
pub struct QueryParams{
    error: String,
    tag: String
}
pub async fn login_form(query: Option<web::Query<QueryParams>>,
    secret: web::Data<HmacSecret>) -> HttpResponse{
    let error_html = String::new();
    for m in flash_messages.iter().filter(|m| m.level() == Level::Error){
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let mut response = HttpResponse::Ok().content_type(ContentType::html())
        .content_type(ContentType::html())
        .body(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>
<body>
    {error_html}
    <form action="/login" method="post">
        <label>Username
            <input type="text" placeholder="Enter username" name="username">
        </label>
        <label>Password
            <input type="password" placeholder="Enter password" name="password">
        </label>
        <button type="submit">Login</button>
    </form>
</body>
</html>"#
        ));
    response
        .add_removal_cookie(&Cookie::new("_flash", ""))
        unwrap();
    response
}

impl QueryParams{
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!(
            "error={}",
            urlencoding::Encoded::new(&self.error)
        );

        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
            secret.0.expose_secret().as_bytes()
        ).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;
        Ok(self.error)
        
    }
}