use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use minijinja::context;
use sea_orm::EntityTrait;
use serde::Deserialize;

use crate::{router::AppState, auth::user::AuthSession};

use crate::auth::user::ensure_valid_access_token;
use crate::entities::user::Entity as UserEntity;
use crate::pco::person::{PeoplePage, get_people};

#[derive(Deserialize)]
pub struct PeopleQuery {
    pub offset: Option<usize>,
    pub name: Option<String>,
}

pub async fn search(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
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
    let offset = 0;
    let per_page = 25;
    let people_page = get_people(&user.access_token, 1, per_page, None)
        .await
        .unwrap_or_else(|_| PeoplePage {
            people: vec![],
            total_count: 0,
            count: 0,
            page: 1,
        });
    let has_more = people_page.count + offset < people_page.total_count;
    let next_offset = offset + per_page;
    let tmpl = state.templates.get_template("search.html").unwrap();
    let html = tmpl
        .render(context! {
            people => people_page.people,
            has_more => has_more,
            next_offset => next_offset,
            name => "",
        })
        .unwrap();
    Html(html).into_response()
}

pub async fn search_partial(
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
    let per_page = 25;
    let people_page = get_people(
        &user.access_token,
        offset / per_page + 1,
        per_page,
        query.name.clone(),
    )
    .await
    .unwrap_or_else(|_| PeoplePage {
        people: vec![],
        total_count: 0,
        count: 0,
        page: offset / per_page + 1,
    });
    let has_more = offset + people_page.count < people_page.total_count;
    let next_offset = offset + per_page;
    let tmpl = state
        .templates
        .get_template("people_table_rows.html")
        .unwrap();
    let html = tmpl
        .render(context! {
            people => people_page.people,
            has_more => has_more,
            next_offset => next_offset,
            name => query.name.clone().unwrap_or_default(),
        })
        .unwrap();
    Html(html).into_response()
}
