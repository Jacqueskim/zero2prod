use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;
use unicode_segmentation::UnicodeSegmentation;
use crate::domain::{NewSubscriber, SubscriberName, SubscriberEmail};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use rand::distributions::Alphanumeric;
use rand::{thread_rng,Rng};
use sqlx::{Postgres,Transaction,Executor};
use actix_web::ResponseError;
use actix_web::http::StatusCode;
use anyhow::Context;

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError{
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{
        write!(f,
        "A database error was encountered while \
        tring to store a sbuscription token.")
    }
}
impl std::fmt::Debug for StoreTokenError{
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result{
        error_chain_fmt(self,f)
    }
}
impl std::error::Error for StoreTokenError{
    fn source(&self)->Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}
fn error_chain_fmt(
    e:&impl std::error::Error,
    f:&mut std::fmt::Formatter<'_>,
) ->std::fmt::Result{
    writeln!(f,"{}\n",e)?;
    let mut current = e.source();
    while let Some(cause) = current{
        writeln!(f, "Caused by\n\t{}",cause)?;
        current = cause.source();
    }
    Ok(())
}
impl ResponseError for StoreTokenError{}

impl ResponseError for SubscribeError{
    fn status_code(&self)->StatusCode{
        match self{
            SubscribeError::ValidationError(_) =>StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode ::INTERNAL_SERVER_ERROR,
        }
    }
}
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}
impl TryFrom<FormData> for NewSubscriber{
    type Error = String;

    fn try_from(value:FormData)->Result<Self, Self::Error>{
        let name = SubscriberName::parse(value.name)?;
    let email= SubscriberEmail::parse(value.email)?;

    Ok(NewSubscriber{email,name})
    }
}


#[tracing::instrument(name = "Adding a new subscriber", skip(form,pool,email_client,base_url),
    fields(
        subscriber_email =%form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(form:web::Form<FormData>,
    pool: web::Data<PgPool>, email_client:web::Data<EmailClient>, base_url:web::Data<ApplicationBaseUrl>) ->Result<HttpResponse, SubscribeError> {

    let new_subscriber = form.0.try_into()
        .map_err(SubscribeError::ValidationError)?;
    let mut transaction =  pool.begin().await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let subscriber_id =  insert_subscriber(&mut transaction,&new_subscriber).await
        .context("Failed to insert new subscriber in the database.")?;
    let subscription_token = generate_subscription_token();
     store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;
     transaction.commit().await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;   
    
     send_confirmation_email(&email_client, new_subscriber, &base_url.0, &subscription_token)
        .await
        .context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().finish())

}

#[derive(thiserror::Error)]
pub enum SubscribeError{
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error)
  
}



impl std::fmt::Debug for SubscribeError{
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>) ->std::fmt::Result{
        error_chain_fmt(self,f)
    }
}

#[tracing::instrument(
    name="Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction:&mut Transaction<'_, Postgres>,
    subscriber_id:Uuid,
    subscription_token:&str,
)->Result<(),StoreTokenError>{
    let query =sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
        );
        transaction.execute(query)
        .await
        .map_err(|e|{
            tracing::error!("Failed to execute query: {:?}", e);
    StoreTokenError(e)
})?;
Ok(())

}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url,subscription_token)
)]
pub async fn send_confirmation_email(
    email_client:&EmailClient,
    new_subscriber:NewSubscriber,
    base_url:&str,
subscription_token:&str) ->Result<(), reqwest::Error>{
        let confirmation_link = format!("{}/subscriptions/confirm?subscription_token={}", base_url, subscription_token);
        let  plain_body = format!(
            "Welcome to our newsletter! \nVisit {} to confirm your subscription.",
            confirmation_link
        );
        let html_body = format!("
            Welcome to our newletter! <br/>
            Click <a href=\"{}\">here</a> to confirm your subscription.",
            confirmation_link
        );
     email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            &html_body,
            &plain_body,
        )
        .await
        
    }


#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query=sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );
    transaction.execute(query)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

pub fn is_valid_name(s:&str) ->bool {
    let is_empty_or_whitespace = s.trim().is_empty();
    let is_too_long = s.graphemes(true).count() > 256;

    let forbidden_chracters = ['/', '(', ')', '"', '<', '>', '\\', '{','}'];
    let contains_forbidden_characters = s
        .chars()
        .any(|g| forbidden_chracters.contains(&g));

    !(is_empty_or_whitespace|| is_too_long || contains_forbidden_characters)
}

fn generate_subscription_token()->String{
    let mut rng = thread_rng();
    std::iter::repeat_with(||rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}