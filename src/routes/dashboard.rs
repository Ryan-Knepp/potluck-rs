use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::{
    AppState,
    entities::potluck_series::{self, Entity as PotluckSeries},
};

pub async fn dashboard(State(state): State<AppState>) -> impl IntoResponse {
    let now = Utc::now().naive_utc();

    // Get active series (where end_date >= today)
    let active_series = PotluckSeries::find()
        .filter(potluck_series::Column::EndDate.gte(now.date()))
        .order_by_asc(potluck_series::Column::StartDate)
        .one(&state.db)
        .await
        .unwrap_or(None);

    // Get past series
    let past_series = PotluckSeries::find()
        .filter(potluck_series::Column::EndDate.lt(now.date()))
        .order_by_desc(potluck_series::Column::EndDate)
        .limit(5)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let tmpl = state.templates.get_template("dashboard.html").unwrap();
    let html = tmpl
        .render(minijinja::context! {
            active_series => active_series,
            past_series => past_series,
        })
        .unwrap();

    Html(html)
}
