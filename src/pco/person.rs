use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const BASE_URL: &str = "https://api.planningcenteronline.com/people/v2/";
const INCLUDED: &str = "include=addresses,emails,households,organization,phone_numbers";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonData {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub email: Option<String>,
    pub address: Option<Value>, // Store full address JSON
    pub phone: Option<String>,
    pub household: Option<HouseholdInfo>,
    pub organization: Option<OrganizationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseholdInfo {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInfo {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PCOMeResponse {
    pub data: PCOResource,
    pub included: Vec<PCOResource>,
    pub meta: Meta,
}

#[derive(Debug, Deserialize)]
pub struct PCOPersonResponse {
    pub data: Vec<PCOResource>,
    pub included: Vec<PCOResource>,
    pub meta: Meta,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[allow(dead_code)]
pub struct PCOResource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: serde_json::Value,
    pub relationships: Option<serde_json::Value>,
    pub links: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Meta {
    pub can_include: Vec<String>,
    pub parent: Parent,
    pub total_count: Option<usize>,
    pub count: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Parent {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
}

#[derive(Serialize, Debug)]
pub struct PeoplePage {
    pub people: Vec<PersonData>,
    pub total_count: usize,
    pub count: usize,
    pub page: usize,
}

pub async fn get_user_info(access_token: &String) -> Result<Option<PersonData>, reqwest::Error> {
    let data = reqwest::Client::new()
        .get(format!("{BASE_URL}/me?{INCLUDED}"))
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<PCOMeResponse>()
        .await?;

    Ok(parse_complete_response(data))
}

fn parse_complete_response(response: PCOMeResponse) -> Option<PersonData> {
    // Create maps to store included data
    let mut addresses: HashMap<String, Value> = HashMap::new();
    let mut emails: HashMap<String, String> = HashMap::new();
    let mut phones: HashMap<String, String> = HashMap::new();
    let mut organizations: HashMap<String, OrganizationInfo> = HashMap::new();
    let mut households: HashMap<String, HouseholdInfo> = HashMap::new();

    // Process included items
    for item in response.included {
        let item_type = item.resource_type;
        let item_id = item.id;

        match item_type {
            val if val == "Address" => {
                // Store the full attributes JSON object
                addresses.insert(item_id, item.attributes);
            }
            val if val == "Email" => {
                if let Some(address) = item.attributes["address"].as_str() {
                    emails.insert(item_id, address.to_string());
                }
            }
            val if val == "PhoneNumber" => {
                if let Some(number) = item.attributes["number"].as_str() {
                    phones.insert(item_id, number.to_string());
                }
            }
            val if val == "Organization" => {
                organizations.insert(
                    item_id.clone(),
                    OrganizationInfo {
                        id: item_id,
                        name: item.attributes["name"].as_str().unwrap_or("").to_string(),
                        avatar_url: item.attributes["avatar_url"]
                            .as_str()
                            .map(|s| s.to_string()),
                    },
                );
            }
            val if val == "Household" => {
                households.insert(
                    item_id.clone(),
                    HouseholdInfo {
                        id: item_id,
                        name: item.attributes["name"].as_str().unwrap_or("").to_string(),
                        avatar: item.attributes["avatar"].as_str().map(|s| s.to_string()),
                    },
                );
            }
            _ => {}
        }
    }

    parse_person_resource(
        response.data,
        organizations,
        addresses,
        emails,
        phones,
        households,
    )
}

fn parse_person_resource(
    person: PCOResource,
    organizations: HashMap<String, OrganizationInfo>,
    addresses: HashMap<String, Value>,
    emails: HashMap<String, String>,
    phones: HashMap<String, String>,
    households: HashMap<String, HouseholdInfo>,
) -> Option<PersonData> {
    let person_id = person.id;
    let attributes = person.attributes;
    let relationships = person.relationships;

    // Extract name and avatar directly
    let name = attributes["name"].as_str().unwrap_or("").to_string();
    let avatar = attributes["avatar"].as_str().map(|s| s.to_string());

    // Find address, email, phone, etc.
    let mut person_address = None;
    let mut person_email = None;
    let mut person_phone = None;
    let mut person_organization = None;
    let mut person_household = None;

    // Process address relationships
    if let Some(relationships) = relationships {
        if let Some(address_data) = relationships["addresses"]["data"].as_array() {
            if !address_data.is_empty() {
                if let Some(addr_id) = address_data[0]["id"].as_str() {
                    person_address = addresses.get(addr_id).cloned();
                }
            }
        }
        if let Some(address_data) = relationships["addresses"]["data"].as_array() {
            if !address_data.is_empty() {
                if let Some(addr_id) = address_data[0]["id"].as_str() {
                    person_address = addresses.get(addr_id).cloned();
                }
            }
        }

        // Process email relationships
        if let Some(email_data) = relationships["emails"]["data"].as_array() {
            if !email_data.is_empty() {
                if let Some(email_id) = email_data[0]["id"].as_str() {
                    person_email = emails.get(email_id).cloned();
                }
            }
        }

        // Process phone relationships
        if let Some(phone_data) = relationships["phone_numbers"]["data"].as_array() {
            if !phone_data.is_empty() {
                if let Some(phone_id) = phone_data[0]["id"].as_str() {
                    person_phone = phones.get(phone_id).cloned();
                }
            }
        }

        // Process organization relationship
        if let Some(org_id) = relationships["organization"]["data"]["id"].as_str() {
            person_organization = organizations.get(org_id).cloned();
        }

        // Process household relationships
        if let Some(household_data) = relationships["households"]["data"].as_array() {
            if !household_data.is_empty() {
                if let Some(household_id) = household_data[0]["id"].as_str() {
                    person_household = households.get(household_id).cloned();
                }
            }
        }
    }

    // Create a person data object
    Some(PersonData {
        id: person_id,
        name,
        avatar,
        email: person_email,
        address: person_address,
        phone: person_phone,
        organization: person_organization,
        household: person_household,
    })
}

pub async fn get_people(
    access_token: &str,
    page: usize,
    per_page: usize,
    name: Option<String>,
) -> Result<PeoplePage, reqwest::Error> {
    let offset = (page - 1) * per_page;
    let mut url = format!(
        "{BASE_URL}people?{INCLUDED}&per_page={}&offset={}&order=last_name&where[status]=active",
        per_page, offset
    );
    if let Some(name) = name {
        url.push_str(&format!(
            "&where[search_name]={}",
            utf8_percent_encode(&name, NON_ALPHANUMERIC)
        ));
    }
    let response = reqwest::Client::new()
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<PCOPersonResponse>()
        .await?;

    // Build lookup maps from included
    let mut addresses: HashMap<String, Value> = HashMap::new();
    let mut emails: HashMap<String, String> = HashMap::new();
    let mut phones: HashMap<String, String> = HashMap::new();
    let mut organizations: HashMap<String, OrganizationInfo> = HashMap::new();
    let mut households: HashMap<String, HouseholdInfo> = HashMap::new();

    for item in response.included {
        let item_type = item.resource_type;
        let item_id = item.id;

        match item_type.as_str() {
            "Address" => {
                addresses.insert(item_id, item.attributes);
            }
            "Email" => {
                if let Some(address) = item.attributes["address"].as_str() {
                    emails.insert(item_id, address.to_string());
                }
            }
            "PhoneNumber" => {
                if let Some(number) = item.attributes["number"].as_str() {
                    phones.insert(item_id, number.to_string());
                }
            }
            "Organization" => {
                organizations.insert(
                    item_id.clone(),
                    OrganizationInfo {
                        id: item_id,
                        name: item.attributes["name"].as_str().unwrap_or("").to_string(),
                        avatar_url: item.attributes["avatar_url"]
                            .as_str()
                            .map(|s| s.to_string()),
                    },
                );
            }
            "Household" => {
                households.insert(
                    item_id.clone(),
                    HouseholdInfo {
                        id: item_id,
                        name: item.attributes["name"].as_str().unwrap_or("").to_string(),
                        avatar: item.attributes["avatar"].as_str().map(|s| s.to_string()),
                    },
                );
            }
            _ => {}
        }
    }

    // Parse each person in the response
    let mut people = Vec::new();
    for person in response.data {
        if let Some(person_data) = parse_person_resource(
            person,
            organizations.clone(),
            addresses.clone(),
            emails.clone(),
            phones.clone(),
            households.clone(),
        ) {
            people.push(person_data);
        }
    }

    let total_count = response.meta.total_count.unwrap_or(0);
    let count = response.meta.count.unwrap_or(0);

    Ok(PeoplePage {
        people,
        total_count,
        count,
        page,
    })
}
