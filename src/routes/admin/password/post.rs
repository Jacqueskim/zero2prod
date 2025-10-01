use actix_web::{HttpResponse, web};
use secrecy::Secret;
use crate::session_state::TypedSession;
use crate::utils::{e500,see_other};
use actix_web_flash_messages::FlashMessage;
use crate::routes::admin::dashboard::get_username;
use sqlx::PgPool;
use crate::authentication::{validate_credentials,AuthError, Credentials};
use actix_web::error::InternalError;
use uuid::Uuid;
use crate::autentication::UserId;
#[derive(serde::Deserialize)]
pub struct FormData{
    current_password:Secret<String>,
    new_password:Secret<String>,
    new_password_check:Secret<String>,
}
async fn reject_anonymous_users(session: &TypedSession)->
    Result<Uuid, actix_web::Error>{
        match session.get_user_id().map_error(e500)? {
            Some(user_id) => Ok(user_id),
            None => 
            let response = see_other("/login");
            let e =anyhow::anyhow!("The user has not logged in");
            Err(InternalError::from_response(e,response).into())
        }
    }
pub async fn change_password(form: web::Form<FormData>, user_id:web::ReqData<UserId>, pool:web::Data<PgPool>) ->
Result<HttpResponse, actix_web::Error>{
        let user_id = user_id.into_inner();
        if user_id.is_none(){
            return Ok(see_other("/login"));
        };
        let user_id = user_id.unwrap();
        if form.new_password.expose_secret() != form.new_password_check.expose_secret(){
            FlashMessage::error(
                "You entered two different new passwords - the field values must match.",
            )
            .send();
        }
        let username = get_username(*user_id, &pool).await.map_err(e500)?;
        let credentials = Credentials{
            username,
            password: form.0.current_password,
        };
        if let Err(e) = validate_credentials(credentials, &pool).await{
            return match e{
                AuthError::InvalidCredentials(_) =>{
                    FlashMessage::error("The current password is incorrect.").send();
                    Ok(see_other("/admin/password"))
                }
                AuthError::UnexpectedError(_) => Err(e500(e).into()),
            }
        }
        crate::authentication::change_password(*user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;
        FlashMessage::info("Your password has been changed.").send();
        Ok(see_other("/admin/password"))
}