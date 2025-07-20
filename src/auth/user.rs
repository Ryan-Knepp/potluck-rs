use crate::OauthClient;
use crate::entities::user::{
    ActiveModel as UserActiveModel, Entity as UserEntity, Model as UserModel,
};
use async_session::async_trait;
use axum_login::{AuthUser, AuthnBackend, UserId};
use chrono::TimeDelta;
use chrono::Utc;
use oauth2::{AuthorizationCode, TokenResponse};
use oauth2::{CsrfToken, HttpClientError, Scope, basic::BasicRequestTokenError};
use reqwest::Url;
use sea_orm::{ActiveValue::*, IntoActiveModel, prelude::*};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;
use tokio::spawn;
use tracing::debug;

use crate::entities::{household, organization, person, prelude::*, user};
use crate::pco::person::get_user_info;

impl AuthUser for user::Model {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.access_token.as_bytes()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub code: String,
    pub old_state: CsrfToken,
    pub new_state: CsrfToken,
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error(transparent)]
    Seaorm(sea_orm::DbErr),

    #[error("User not found")]
    UnknownUser,

    #[error("Organization not found")]
    UnknownOrganization,

    #[error(transparent)]
    Reqwest(reqwest::Error),

    #[error(transparent)]
    OAuth2(BasicRequestTokenError<HttpClientError<reqwest::Error>>),
}

#[derive(Debug, Clone)]
pub struct Backend {
    db: DatabaseConnection,
    client: OauthClient,
}

impl Backend {
    pub fn new(db: DatabaseConnection, client: OauthClient) -> Self {
        Self { db, client }
    }

    pub fn authorize_url(&self) -> (Url, CsrfToken) {
        self.client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("people".to_string()))
            .url()
    }
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = user::Model;
    type Credentials = Credentials;
    type Error = BackendError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        // Ensure the CSRF state has not been tampered with.
        if creds.old_state.secret() != creds.new_state.secret() {
            return Ok(None);
        };

        let http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        // Process authorization code, expecting a token response back.
        let token_res = self
            .client
            .exchange_code(AuthorizationCode::new(creds.code))
            .request_async(&http_client)
            .await
            .map_err(Self::Error::OAuth2)?;

        let access_token = token_res.access_token().secret();
        let refresh_token = token_res.refresh_token().map(|t| t.secret().to_string());
        let expires_in = token_res
            .expires_in()
            .unwrap_or_else(|| std::time::Duration::from_secs(7200));
        let token_expires_at =
            chrono::Utc::now().naive_utc() + TimeDelta::seconds(expires_in.as_secs() as i64);
        let user_info = get_user_info(access_token)
            .await
            .map_err(Self::Error::Reqwest)?;

        if user_info.is_none() {
            return Err(Self::Error::UnknownUser);
        }

        let user_data = user_info.ok_or(Self::Error::UnknownUser)?;

        // Persist user in our database so we can use `get_user`.
        let org = user_data
            .organization
            .ok_or(Self::Error::UnknownOrganization)?;
        let existing_org = Organization::find()
            .filter(organization::Column::PcoId.eq(org.id.clone()))
            .one(&self.db)
            .await
            .map_err(Self::Error::Seaorm)?;

        debug!("Handling organization");
        let organization = match existing_org {
            Some(existing) => {
                // Update existing org
                let mut org_model: organization::ActiveModel = existing.into();
                org_model.name = Set(org.name);
                org_model.avatar_url = Set(org.avatar_url);
                org_model
                    .update(&self.db)
                    .await
                    .map_err(Self::Error::Seaorm)?
            }
            None => {
                // Create new org
                let org_model = organization::ActiveModel {
                    pco_id: Set(org.id),
                    name: Set(org.name),
                    avatar_url: Set(org.avatar_url),
                    ..Default::default()
                };
                debug!("Creating new organization: {:?}", org_model);
                org_model
                    .insert(&self.db)
                    .await
                    .map_err(Self::Error::Seaorm)?
            }
        };

        // Start creating the person record
        debug!("Handling person");
        let mut is_new_person = false;
        let person = Person::find()
            .filter(person::Column::PcoId.eq(user_data.id.clone()))
            .one(&self.db)
            .await
            .map_err(Self::Error::Seaorm)?;
        let mut person = match person {
            Some(existing) => {
                let mut person = existing.into_active_model();
                person.name = Set(user_data.name);
                person.avatar_url = Set(user_data.avatar);
                person.email = Set(user_data.email);
                person.phone = Set(user_data.phone);
                person.address = Set(user_data.address.unwrap_or_default());
                person.updated_at = Set(chrono::Utc::now().naive_utc());
                person
            }
            None => {
                // Create new person
                is_new_person = true;
                person::ActiveModel {
                    id: NotSet,
                    organization_id: Set(organization.id),
                    pco_id: Set(user_data.id),
                    name: Set(user_data.name),
                    avatar_url: Set(user_data.avatar),
                    email: Set(user_data.email),
                    phone: Set(user_data.phone),
                    address: Set(user_data.address.unwrap_or_default()),
                    can_host: Set(false),
                    is_signed_up: Set(false),
                    is_child: Set(false),
                    household_id: Set(None),
                    created_at: Set(chrono::Utc::now().naive_utc()),
                    updated_at: Set(chrono::Utc::now().naive_utc()),
                }
            }
        };

        let household: Option<household::Model> = match user_data.household {
            Some(household) => {
                let existing_household = Household::find()
                    .filter(household::Column::PcoId.eq(household.id.clone()))
                    .one(&self.db)
                    .await
                    .map_err(Self::Error::Seaorm)?;

                let household = match existing_household {
                    Some(existing) => {
                        // Update existing household
                        let mut household_model: household::ActiveModel = existing.into();
                        household_model.name = Set(household.name);
                        household_model.avatar_url = Set(household.avatar);
                        household_model
                            .update(&self.db)
                            .await
                            .map_err(Self::Error::Seaorm)?
                    }
                    None => {
                        // Create new household
                        let household_model = household::ActiveModel {
                            organization_id: Set(organization.id),
                            pco_id: Set(household.id),
                            name: Set(household.name),
                            avatar_url: Set(household.avatar),
                            ..Default::default()
                        };
                        debug!("Creating new household: {:?}", household_model);
                        household_model
                            .insert(&self.db)
                            .await
                            .map_err(Self::Error::Seaorm)?
                    }
                };
                Some(household)
            }
            None => None,
        };

        debug!("Creating new person: {:?}", person);
        person.household_id = match household {
            Some(ref h) => Set(Some(h.id)),
            None => NotSet,
        };

        let person = match is_new_person {
            true => person.insert(&self.db).await.map_err(Self::Error::Seaorm)?,
            false => person.update(&self.db).await.map_err(Self::Error::Seaorm)?,
        };

        let user = user::Entity::find()
            .filter(user::Column::PersonId.eq(person.id))
            .filter(user::Column::OrganizationId.eq(organization.id))
            .one(&self.db)
            .await
            .map_err(Self::Error::Seaorm)?;

        debug!("Handling user");
        let user = match user {
            Some(user) => {
                let mut user_model = user.into_active_model();
                user_model.access_token = Set(access_token.clone());
                user_model.refresh_token = Set(refresh_token.clone());
                user_model.token_expires_at = Set(token_expires_at);
                user_model
                    .update(&self.db)
                    .await
                    .map_err(Self::Error::Seaorm)?
            }
            None => {
                let user_model = user::ActiveModel {
                    person_id: Set(person.id),
                    organization_id: Set(organization.id),
                    access_token: Set(access_token.clone()),
                    refresh_token: Set(refresh_token.clone()),
                    token_expires_at: Set(token_expires_at),
                    ..Default::default()
                };
                debug!("Creating new user: {:?}", user_model);
                user_model
                    .insert(&self.db)
                    .await
                    .map_err(Self::Error::Seaorm)?
            }
        };

        if is_new_person && household.is_some() {
            let db = self.db.clone();
            let pco_id = household.unwrap().pco_id;
            spawn(async move {
                if let Err(e) = get_household_data(pco_id, db).await {
                    tracing::error!("Error getting household data: {:?}", e);
                }
            });
        }

        Ok(Some(user))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user = user::Entity::find()
            .filter(user::Column::Id.eq(*user_id))
            .one(&self.db)
            .await
            .map_err(Self::Error::Seaorm)?;

        if let Some(user) = user {
            Ok(Some(user))
        } else {
            Err(Self::Error::UnknownUser)
        }
    }
}

// We use a type alias for convenience.
//
// Note that we've supplied our concrete backend here.
pub type AuthSession = axum_login::AuthSession<Backend>;

async fn get_household_data(pco_id: String, db: DatabaseConnection) -> Result<(), BackendError> {
    let household = Household::find()
        .filter(household::Column::PcoId.eq(pco_id))
        .one(&db)
        .await
        .map_err(BackendError::Seaorm)?;

    if household.is_none() {
        return Err(BackendError::UnknownUser);
    }

    Ok(())
}

pub async fn ensure_valid_access_token(
    user: &mut UserModel,
    db: &DatabaseConnection,
    oauth_client: &OauthClient,
) -> Result<(), anyhow::Error> {
    if user.token_expires_at < Utc::now().naive_utc() {
        let refresh_token = user
            .refresh_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No refresh token"))?;
        let token_result = oauth_client
            .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
            .request_async(&reqwest::Client::new())
            .await?;

        user.access_token = token_result.access_token().secret().to_string();
        user.refresh_token = token_result.refresh_token().map(|t| t.secret().to_string());
        user.token_expires_at = Utc::now().naive_utc()
            + chrono::Duration::from_std(token_result.expires_in().unwrap()).unwrap();

        // Save updated user to DB
        let mut active_model: UserActiveModel = user.clone().into();
        active_model.access_token = Set(user.access_token.clone());
        active_model.refresh_token = Set(user.refresh_token.clone());
        active_model.token_expires_at = Set(user.token_expires_at);
        UserEntity::update(active_model).exec(db).await?;
    }
    Ok(())
}
