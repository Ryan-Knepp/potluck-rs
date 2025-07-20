use crate::pco::person::PCOResource;
use reqwest;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.planningcenteronline.com/people/v2/households";
const INCLUDED: &str = "include=addresses,emails,households,organization,phone_numbers&filter=without_deceased";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseholdResponse {
    pub data: Vec<PCOResource>,
    pub included: Vec<PCOResource>,
}

pub async fn get_household_people(
    access_token: &String,
    household_id: &String,
) -> Result<HouseholdResponse, reqwest::Error> {
    let url = format!("{}/{}/people?{}", BASE_URL, household_id, INCLUDED);
    let household = reqwest::Client::new()
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<HouseholdResponse>()
        .await?;

    Ok(household)
}