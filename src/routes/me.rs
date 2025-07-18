use crate::entities::{organization, person};
use crate::{router::AppState, auth::user::AuthSession};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use reqwest::StatusCode;
use sea_orm::EntityTrait;

pub async fn me(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    if let Some(user) = auth_session.user {
        // Fetch related person
        let person = person::Entity::find_by_id(user.person_id)
            .one(&state.db)
            .await
            .unwrap_or(None);

        if person.is_none() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        let person = person.unwrap();

        // Fetch related organization
        let organization = organization::Entity::find_by_id(user.organization_id)
            .one(&state.db)
            .await
            .unwrap_or(None);
        if organization.is_none() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        let organization = organization.unwrap();

        let tmpl = state.templates.get_template("me.html").unwrap();
        let html = tmpl
            .render(minijinja::context! {
                name => person.name,
                email => person.email,
                phone => person.phone,
                address => person.address,
                avatar_url => person.avatar_url,
                organization_name => organization.name,
                created_at => user.created_at.format("%Y-%m-%d").to_string(),
            })
            .unwrap();
        Html(html).into_response()
    } else {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
