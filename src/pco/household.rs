use reqwest;

use super::person::{parse_person_resource, HouseholdInfo};
use super::{process_included, PCOPersonResponse, INCLUDED, BASE_URL};

fn parse_complete_response(response: PCOPersonResponse) -> Option<HouseholdInfo> {
    let (addresses, emails, phones, organizations, households) = process_included(response.included);

    let household_id = response.meta.parent.id.clone();
    let mut household_info = households.get(&household_id).cloned().unwrap();

    let people = response
        .data
        .into_iter()
        .filter_map(|person_resource| {
            parse_person_resource(
                person_resource,
                organizations.clone(),
                addresses.clone(),
                emails.clone(),
                phones.clone(),
                households.clone(),
            )
        })
        .collect();

    household_info.people = Some(people);

    Some(household_info)
}

pub async fn get_household_people(
    access_token: &str,
    household_id: &str,
) -> Result<Option<HouseholdInfo>, reqwest::Error> {
    let url = format!(
        "{}households/{}/people?{}",
        BASE_URL, household_id, INCLUDED
    );
    let response = reqwest::Client::new()
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await?        .json::<PCOPersonResponse>()
        .await?;

    Ok(parse_complete_response(response))
}
