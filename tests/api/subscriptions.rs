use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock,ResponseTemplate};


#[tokio::test]
async fn subscribe_returns_200_for_valid_data() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
   
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(200,response.status().as_u16());
}

#[tokio::test]
async fn subscriber_persists_the_new_subscriber(){
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    app.post_subscriptions(body.into()).await;
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)

        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status,"pending_confirmation");
}

#[tokio::test]
async fn susbscribe_returns_400_when_data_is_missing() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email")
    ];

    for (invalid_body, error_message) in test_cases{
        let response = app.post_subscriptions(invalid_body.into()).await;
        assert_eq!(400, response.status().as_u16(),
            "The API did not fail with 400 Bad REquest when the 
            payload was {}.", error_message);
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid(){
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula@email=definitely_not-an-email","invalid email")
    ];
    for(body, description) in test_cases{
        let response = app.post_subscriptions(body.into()).await;
        assert_eq!(400,response.status().as_u16(), "The API did not return a 400 OK when the payload was {}.",
    description);
    }
    
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data(){
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    app.post_subscriptions(body.into()).await;   
}
#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link(){
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&app.email_server)
    .await;
app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    assert_eq!(confirmation_links.html,confirmation_links.plain_text);
}

#[tokio::test]
async fn subscriber_fails_if_there_is_a_fatal_database_error(){
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(response.status().as_u16(),500);
}