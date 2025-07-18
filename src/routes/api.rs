use crate::auth::user::ensure_valid_access_token;
use crate::entities::user::Entity as UserEntity;
use crate::pco::person::get_people;
use crate::{router::AppState, auth::user::AuthSession};
use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::StatusCode;
use sea_orm::EntityTrait;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PeopleQuery {
    pub offset: Option<usize>,
    pub name: Option<String>,
}

/// Protected API endpoint to fetch paginated people from Planning Center
pub async fn api_people(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Query(query): Query<PeopleQuery>,
) -> impl IntoResponse {
    let user_id = match auth_session.user {
        Some(u) => u.id,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // Fetch user from DB
    let mut user = match UserEntity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // Ensure access token is valid (refresh if needed)
    if ensure_valid_access_token(&mut user, &state.db, &state.client).await.is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let offset = query.offset.unwrap_or(0);
    let per_page = 25;
    let page = offset / per_page + 1;
    match get_people(&user.access_token, page, per_page, query.name.clone()).await {
        Ok(people) => Json(people).into_response(),
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

/// Proxy endpoint to return raw Planning Center people API JSON (per_page=5)
pub async fn api_pco(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Query(query): Query<PeopleQuery>,
) -> impl IntoResponse {
    let user_id = match auth_session.user {
        Some(u) => u.id,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let mut user = match UserEntity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if ensure_valid_access_token(&mut user, &state.db, &state.client).await.is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let offset = query.offset.unwrap_or(0);
    let per_page = 5;
    let mut url = format!(
        "https://api.planningcenteronline.com/people/v2/people?per_page={}&offset={}&order=last_name&where[status]=active",
        per_page, offset
    );
    if let Some(name) = &query.name {
        url.push_str(&format!(
            "&where[search_name]={}",
            utf8_percent_encode(name, NON_ALPHANUMERIC)
        ));
    }
    let client = reqwest::Client::new();
    let resp = client.get(url).bearer_auth(&user.access_token).send().await;
    match resp {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(json) => Json(json).into_response(),
            Err(_) => StatusCode::BAD_GATEWAY.into_response(),
        },
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}
