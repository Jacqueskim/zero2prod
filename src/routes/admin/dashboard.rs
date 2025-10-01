use actix_web::{web,HttpResponse};
use actix_session::Session;
use uuid::Uuid;
use actix_web_http::header::ContentType;
use actix_web::web;
use anyhow::Context;
use sqlx::PgPool;
use crate::session_state::TypedSession;
use actix_web::http::header::LOCATION;
use crate::utils::e500;

pub async fn admin_dashboard(session:Session)->HttpResponse{
    let _username = if let Some(user_id) = Session
        .get::<Uuid>("user_id")
        .map_err(e500)?
    {
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
                <title>Admin Dashboard</title>
            </head>
            <body>
                <p>Welcome {username}!</p>
                <p>Available actions:</p>
                <ol>
                    <li><a href="/admin/password">Change password</a></li>
                    <li>
                        <form name="logoutForm" action="/admin/logout" method="post">
                            <button type="submit" value="Logout>
                        </form>
                    </li>
                </ol>
                
            </body>
            </html>"#
        )))
}

#[tracing::instrument(name= "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool)->
    Result<String, anyhow::Error>{
        let row = sqlx::query!(
            r#"SELECT username FROM users WHERE id = $1"#,
            user_id
        )
        .fetch_one(pool)
        .await
        .context("Failed to perform a query to retrieve a username.")?;
        Ok(row.username)
    }
