use axum::{
    Router,
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use minijinja::context;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Deserialize;

use crate::{
    auth::user::AuthSession,
    entities::{household, person},
    router::AppState,
};

#[derive(Deserialize)]
pub struct PeopleParams {
    tab: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(all_people))
        .route(
            "/household/{id}/toggle-active",
            post(toggle_household_active),
        )
        .route("/person/{id}/toggle-active", post(toggle_person_active))
        .route("/household/{id}/toggle-host", post(toggle_household_host))
        .route("/person/{id}/toggle-host", post(toggle_person_host))
}

pub async fn all_people(
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    if auth_session.user.is_none() {
        return Redirect::to("/login").into_response();
    }

    let households = household::Entity::find()
        .order_by_asc(household::Column::Name)
        .find_with_related(person::Entity)
        .all(&state.db)
        .await
        .unwrap();

    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.is_null())
        .order_by_asc(person::Column::Name)
        .all(&state.db)
        .await
        .unwrap();

    let tmpl = state.templates.get_template("people.html").unwrap();
    let html = tmpl
        .render(context! { households => households, people => people, active => "people", tab => "active" })
        .unwrap();
    Html(html).into_response()
}

async fn render_people_list(state: AppState, tab: String) -> impl IntoResponse {
    let households = household::Entity::find()
        .order_by_asc(household::Column::Name)
        .find_with_related(person::Entity)
        .all(&state.db)
        .await
        .unwrap();

    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.is_null())
        .order_by_asc(person::Column::Name)
        .all(&state.db)
        .await
        .unwrap();

    let tmpl = state.templates.get_template("_people_list.html").unwrap();
    let html = tmpl
        .render(context! { households => households, people => people, tab => tab })
        .unwrap();
    Html(html).into_response()
}

pub async fn toggle_household_active(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<PeopleParams>,
) -> impl IntoResponse {
    let mut household: household::ActiveModel = household::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap()
        .into();

    household.is_signed_up = Set(!household.is_signed_up.clone().unwrap());
    household.update(&state.db).await.unwrap();

    render_people_list(state, params.tab.unwrap_or_else(|| "active".to_string())).await
}

pub async fn toggle_person_active(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<PeopleParams>,
) -> impl IntoResponse {
    let mut person: person::ActiveModel = person::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap()
        .into();

    person.is_signed_up = Set(!person.is_signed_up.clone().unwrap());
    person.update(&state.db).await.unwrap();

    render_people_list(state, params.tab.unwrap_or_else(|| "active".to_string())).await
}

pub async fn toggle_household_host(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<PeopleParams>,
) -> impl IntoResponse {
    let mut household: household::ActiveModel = household::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap()
        .into();

    household.can_host = Set(!household.can_host.clone().unwrap());
    household.update(&state.db).await.unwrap();

    render_people_list(state, params.tab.unwrap_or_else(|| "active".to_string())).await
}

pub async fn toggle_person_host(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<PeopleParams>,
) -> impl IntoResponse {
    let mut person: person::ActiveModel = person::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap()
        .into();

    person.can_host = Set(!person.can_host.clone().unwrap());
    person.update(&state.db).await.unwrap();

    render_people_list(state, params.tab.unwrap_or_else(|| "active".to_string())).await
}