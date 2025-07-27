use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use minijinja::context;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait,
};
use serde::Deserialize;

use crate::{auth::user::AuthSession, router::AppState};

use crate::auth::user::ensure_valid_access_token;
use crate::entities::user::Entity as UserEntity;
use crate::entities::{household, organization, person};
use crate::pco::household::get_household_people;
use crate::pco::person::{get_people, get_person, PeoplePage};

#[derive(Deserialize)]
pub struct PeopleQuery {
    pub offset: Option<usize>,
    pub name: Option<String>,
}

async fn get_people_with_signup_status(
    state: &AppState,
    user: &mut crate::entities::user::Model,
    page: usize,
    per_page: usize,
    name: Option<String>,
) -> Result<PeoplePage, String> {
    if ensure_valid_access_token(user, &state.db, &state.client)
        .await
        .is_err()
    {
        return Err("Invalid token".to_string());
    }

    let mut people_page = get_people(&user.access_token, page, per_page, name.clone())
        .await
        .map_err(|e| e.to_string())?;

    let pco_ids: Vec<String> = people_page
        .people
        .iter()
        .map(|p| p.id.clone())
        .collect();

    let persons_in_db = person::Entity::find()
        .filter(person::Column::PcoId.is_in(pco_ids))
        .all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let household_ids: Vec<String> = people_page
        .people
        .iter()
        .filter_map(|p| p.household.as_ref().map(|h| h.id.clone()))
        .collect();

    let households_in_db = household::Entity::find()
        .filter(household::Column::PcoId.is_in(household_ids))
        .all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    for person_data in &mut people_page.people {
        if let Some(person_in_db) = persons_in_db.iter().find(|p| p.pco_id == person_data.id) {
            person_data.is_signed_up = person_in_db.is_signed_up;
        }
        if let Some(household_info) = &mut person_data.household {
            if let Some(household_in_db) = households_in_db
                .iter()
                .find(|h| h.pco_id == household_info.id)
            {
                household_info.is_signed_up = Some(household_in_db.is_signed_up);
            }
        }
    }

    Ok(people_page)
}

pub async fn search(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    let user_id = match auth_session.user {
        Some(u) => u.id,
        None => return (StatusCode::UNAUTHORIZED, "No user session").into_response(),
    };
    // Fetch user from DB
    let mut user = match UserEntity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::UNAUTHORIZED, "User not found").into_response(),
    };

    let per_page = 25;
    let people_page =
        match get_people_with_signup_status(&state, &mut user, 1, per_page, None).await {
            Ok(page) => page,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        };

    let has_more = people_page.count < people_page.total_count;
    let next_offset = per_page;
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
        None => return (StatusCode::UNAUTHORIZED, "No user session").into_response(),
    };
    let mut user = match UserEntity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::UNAUTHORIZED, "User not found").into_response(),
    };

    let offset = query.offset.unwrap_or(0);
    let per_page = 25;
    let page = offset / per_page + 1;

    let people_page =
        match get_people_with_signup_status(&state, &mut user, page, per_page, query.name.clone())
            .await
        {
            Ok(page) => page,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        };

    let has_more = offset + people_page.count < people_page.total_count;
    let next_offset = offset + per_page;
    let tmpl = state
        .templates
        .get_template("search_table_rows.html")
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

pub async fn sign_up_household(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(household_id): Path<String>,
    Query(query): Query<PeopleQuery>,
) -> impl IntoResponse {
    let user_id = match auth_session.user {
        Some(u) => u.id,
        None => return (StatusCode::UNAUTHORIZED, "No user session").into_response(),
    };
    let mut user = match UserEntity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::UNAUTHORIZED, "User not found").into_response(),
    };
    if ensure_valid_access_token(&mut user, &state.db, &state.client)
        .await
        .is_err()
    {
        return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    }

    let organization_id = user.organization_id;

    let household_info = match get_household_people(&user.access_token, &household_id).await {
        Ok(Some(household_info)) => household_info,
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get household data from PCO",
            )
                .into_response();
        }
    };

    let txn = match state.db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to begin transaction",
            )
                .into_response();
        }
    };

    let household_model = match household::Entity::find()
        .filter(household::Column::PcoId.eq(&household_id))
        .one(&txn)
        .await
    {
        Ok(Some(existing)) => {
            let mut active_model: household::ActiveModel = existing.into();
            active_model.name = Set(household_info.name);
            active_model.avatar_url = Set(household_info.avatar);
            active_model.is_signed_up = Set(true);
            match active_model.update(&txn).await {
                Ok(model) => model,
                Err(_) => {
                    let _ = txn.rollback().await;
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to update household",
                    )
                        .into_response();
                }
            }
        }
        Ok(None) => {
            let new_household = household::ActiveModel {
                pco_id: Set(household_id),
                organization_id: Set(organization_id),
                name: Set(household_info.name),
                avatar_url: Set(household_info.avatar),
                is_signed_up: Set(true),
                can_host: Set(false),
                ..Default::default()
            };
            match new_household.insert(&txn).await {
                Ok(model) => model,
                Err(_) => {
                    let _ = txn.rollback().await;
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to insert household",
                    )
                        .into_response();
                }
            }
        }
        Err(_) => {
            let _ = txn.rollback().await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to query household",
            )
                .into_response();
        }
    };

    if let Some(people) = household_info.people {
        for pco_person in people {
            let existing_person = match person::Entity::find()
                .filter(person::Column::PcoId.eq(&pco_person.id))
                .one(&txn)
                .await
            {
                Ok(person) => person,
                Err(_) => {
                    let _ = txn.rollback().await;
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query person")
                        .into_response();
                }
            };

            if let Some(existing) = existing_person {
                let mut active_model: person::ActiveModel = existing.into();
                active_model.name = Set(pco_person.name);
                active_model.email = Set(pco_person.email);
                active_model.phone = Set(pco_person.phone);
                active_model.address = Set(pco_person.address.unwrap_or_default());
                active_model.avatar_url = Set(pco_person.avatar);
                active_model.is_child = Set(pco_person.is_child);
                active_model.household_id = Set(Some(household_model.id));
                if active_model.update(&txn).await.is_err() {
                    let _ = txn.rollback().await;
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update person")
                        .into_response();
                }
            } else {
                let new_person = person::ActiveModel {
                    pco_id: Set(pco_person.id),
                    organization_id: Set(organization_id),
                    name: Set(pco_person.name),
                    email: Set(pco_person.email),
                    phone: Set(pco_person.phone),
                    address: Set(pco_person.address.unwrap_or_default()),
                    avatar_url: Set(pco_person.avatar),
                    is_signed_up: Set(false),
                    can_host: Set(false),
                    is_child: Set(pco_person.is_child),
                    household_id: Set(Some(household_model.id)),
                    ..Default::default()
                };
                if new_person.insert(&txn).await.is_err() {
                    let _ = txn.rollback().await;
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to insert person")
                        .into_response();
                }
            }
        }
    }

    match txn.commit().await {
        Ok(_) => {
            let offset = query.offset.unwrap_or(0);
            let per_page = 25;
            let page = offset / per_page + 1;
            let mut user = match UserEntity::find_by_id(user.id).one(&state.db).await {
                Ok(Some(u)) => u,
                _ => return (StatusCode::UNAUTHORIZED, "User not found").into_response(),
            };
            let people_page = match get_people_with_signup_status(
                &state,
                &mut user,
                page,
                per_page,
                query.name.clone(),
            )
            .await
            {
                Ok(page) => page,
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            };

            let has_more = offset + people_page.count < people_page.total_count;
            let next_offset = offset + per_page;
            let tmpl = state
                .templates
                .get_template("search_table_rows.html")
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
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to commit transaction",
        )
            .into_response(),
    }
}

pub async fn sign_up_person(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path(person_id): Path<String>,
    Query(query): Query<PeopleQuery>,
) -> impl IntoResponse {
    let user_id = match auth_session.user {
        Some(u) => u.id,
        None => return (StatusCode::UNAUTHORIZED, "No user session").into_response(),
    };
    let mut user = match UserEntity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::UNAUTHORIZED, "User not found").into_response(),
    };
    if ensure_valid_access_token(&mut user, &state.db, &state.client)
        .await
        .is_err()
    {
        return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    }

    let organization_id = user.organization_id;

    let person_data = match get_person(&user.access_token, &person_id).await {
        Ok(Some(person_data)) => person_data,
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get person data from PCO",
            )
                .into_response();
        }
    };

    let txn = match state.db.begin().await {
        Ok(txn) => txn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to begin transaction",
            )
                .into_response();
        }
    };

    // Handle Organization
    let organization_model = if let Some(org_info) = person_data.organization.clone() {
        match organization::Entity::find()
            .filter(organization::Column::PcoId.eq(&org_info.id))
            .one(&txn)
            .await
        {
            Ok(Some(existing)) => {
                let mut active_model: organization::ActiveModel = existing.into();
                active_model.name = Set(org_info.name);
                active_model.avatar_url = Set(org_info.avatar_url);
                match active_model.update(&txn).await {
                    Ok(model) => model,
                    Err(_) => {
                        let _ = txn.rollback().await;
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to update organization",
                        )
                            .into_response();
                    }
                }
            }
            Ok(None) => {
                let new_org = organization::ActiveModel {
                    pco_id: Set(org_info.id),
                    name: Set(org_info.name),
                    avatar_url: Set(org_info.avatar_url),
                    ..Default::default()
                };
                match new_org.insert(&txn).await {
                    Ok(model) => model,
                    Err(_) => {
                        let _ = txn.rollback().await;
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to insert organization",
                        )
                            .into_response();
                    }
                }
            }
            Err(_) => {
                let _ = txn.rollback().await;
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to query organization",
                )
                    .into_response();
            }
        }
    } else {
        // If no organization data from PCO, use the user's organization
        match organization::Entity::find_by_id(organization_id)
            .one(&txn)
            .await
        {
            Ok(Some(org)) => org,
            _ => {
                let _ = txn.rollback().await;
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Organization not found for user",
                )
                    .into_response();
            }
        }
    };

    // Handle Person
    let existing_person = match person::Entity::find()
        .filter(person::Column::PcoId.eq(&person_data.id))
        .one(&txn)
        .await
    {
        Ok(person) => person,
        Err(_) => {
            let _ = txn.rollback().await;
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query person").into_response();
        }
    };

    if let Some(existing) = existing_person {
        let mut active_model: person::ActiveModel = existing.into();
        active_model.name = Set(person_data.name);
        active_model.email = Set(person_data.email);
        active_model.phone = Set(person_data.phone);
        active_model.address = Set(person_data.address.unwrap_or_default());
        active_model.avatar_url = Set(person_data.avatar);
        active_model.is_child = Set(person_data.is_child);
        active_model.is_signed_up = Set(true);
        active_model.organization_id = Set(organization_model.id);
        if active_model.update(&txn).await.is_err() {
            let _ = txn.rollback().await;
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update person").into_response();
        }
    } else {
        let new_person = person::ActiveModel {
            pco_id: Set(person_data.id),
            organization_id: Set(organization_model.id),
            name: Set(person_data.name),
            email: Set(person_data.email),
            phone: Set(person_data.phone),
            address: Set(person_data.address.unwrap_or_default()),
            avatar_url: Set(person_data.avatar),
            is_signed_up: Set(true),
            can_host: Set(false),
            is_child: Set(person_data.is_child),
            household_id: Set(None),
            ..Default::default()
        };
        if new_person.insert(&txn).await.is_err() {
            let _ = txn.rollback().await;
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to insert person").into_response();
        }
    }

    match txn.commit().await {
        Ok(_) => {
            let offset = query.offset.unwrap_or(0);
            let per_page = 25;
            let page = offset / per_page + 1;
            let mut user = match UserEntity::find_by_id(user.id).one(&state.db).await {
                Ok(Some(u)) => u,
                _ => return (StatusCode::UNAUTHORIZED, "User not found").into_response(),
            };
            let people_page = match get_people_with_signup_status(
                &state,
                &mut user,
                page,
                per_page,
                query.name.clone(),
            )
            .await
            {
                Ok(page) => page,
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            };

            let has_more = offset + people_page.count < people_page.total_count;
            let next_offset = offset + per_page;
            let tmpl = state
                .templates
                .get_template("search_table_rows.html")
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
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to commit transaction",
        )
            .into_response(),
    }
}