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
    pool: web::Data<PgPool>, email_client:web::Data<EmailClient>, base_url:web::Data<ApplicationBaseUrl>) ->HttpResponse {

    let new_subscriber = match form.0.try_into() {
        Ok(subscriber)=>subscriber,
        Err(_)=> return HttpResponse::BadRequest().finish(),
    };
    let subscriber_id = match insert_subscriber(&pool,&new_subscriber).await{
        Ok(subscriber_id)=>subscriber_id,
        Err(_)=>return HttpResponse::InternalServerError().finish(),
    };
    let subscription_token = generate_subscription_token();
    if store_token(&pool, subscriber_id, &subscription_token)
        .await
        .is_err()
        {
            return HttpResponse::InternalServerError().finish();
        }
    
    if send_confirmation_email(&email_client, new_subscriber, &base_url.0, &subscription_token)
        .await
        .is_err()
        {
            return HttpResponse::InternalServerError().finish();
        }
    
    
    HttpResponse::Ok().finish()

}

#[tracing::instrument(
    name="Store subscription token in the database",
    skip(subscription_token, pool)
)]
pub async fn store_token(
    pool:&PgPool,
    subscriber_id:Uuid,
    subscription_token:&str,
)->Result<(),sqlx::Error>{
    sqlx::query!(
r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
VALUES ($1, $2)"#,
subscription_token,
subscriber_id
)
.execute(pool)
.await
.map_err(|e|{
    tracing::error!("Failed to execute query: {:?}", e);
    e
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
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
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