use axum::{
    Form, Router,
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use axum_login::tower_sessions::Session;
use oauth2::CsrfToken;
use serde::Deserialize;

use super::user::{AuthSession, Credentials};

pub const NEXT_URL_KEY: &str = "auth.next-url";
pub const CSRF_STATE_KEY: &str = "oauth.csrf-state";

#[derive(Debug, Clone, Deserialize)]
pub struct AuthzResp {
    code: String,
    state: CsrfToken,
}

// This allows us to extract the "next" field from the query string. We use this
// to redirect after log in.
#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub fn router() -> Router {
    Router::new()
        .route("/login", post(self::post::login))
        .route("/login", get(self::get::login))
        .route("/logout", get(self::get::logout))
        .route("/auth/complete", get(self::get::callback))
}

mod post {
    use super::*;

    pub async fn login(
        auth_session: AuthSession,
        session: Session,
        Form(NextUrl { next }): Form<NextUrl>,
    ) -> impl IntoResponse {
        let (auth_url, csrf_state) = auth_session.backend.authorize_url();

        session
            .insert(CSRF_STATE_KEY, csrf_state.secret())
            .await
            .expect("Serialization should not fail.");

        session
            .insert(NEXT_URL_KEY, next)
            .await
            .expect("Serialization should not fail.");

        Redirect::to(auth_url.as_str()).into_response()
    }
}

mod get {
    use super::*;

    pub async fn login(Query(NextUrl { next }): Query<NextUrl>) -> Html<String> {
        let next_param = next.map_or_else(String::new, |url| format!("?next={}", url));

        Html(format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Potluck Login</title>
</head>
<body>
    <h1>Potluck</h1>
    <form action="/login{}" method="post">
        <button type="submit">Login with Planning Center</button>
    </form>
</body>
</html>"#,
            next_param
        ))
    }

    pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.logout().await {
            Ok(_) => Redirect::to("/login").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    pub async fn callback(
        mut auth_session: AuthSession,
        session: Session,
        Query(AuthzResp {
            code,
            state: new_state,
        }): Query<AuthzResp>,
    ) -> impl IntoResponse {
        let Ok(Some(old_state)) = session.get(CSRF_STATE_KEY).await else {
            return StatusCode::BAD_REQUEST.into_response();
        };

        let creds = Credentials {
            code,
            old_state,
            new_state,
        };

        let user = match auth_session.authenticate(creds).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return (StatusCode::UNAUTHORIZED, "You are not allowed here.").into_response();
            }
            Err(_) => {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

        if auth_session.login(&user).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        if let Ok(Some(next)) = session.remove::<String>(NEXT_URL_KEY).await {
            Redirect::to(&next).into_response()
        } else {
            Redirect::to("/me").into_response()
        }
    }
}
