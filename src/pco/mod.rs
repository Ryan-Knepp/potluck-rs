use std::collections::HashMap;
use serde::Deserialize;
use serde_json::Value;
use crate::pco::person::{HouseholdInfo, OrganizationInfo};

pub mod person;
pub mod household;

const BASE_URL: &str = "https://api.planningcenteronline.com/people/v2/";
const INCLUDED: &str = "include=addresses,emails,households,organization,phone_numbers";

#[derive(Debug, Deserialize)]
pub struct PCOPersonResponse {
    pub data: Vec<PCOResource>,
    pub included: Vec<PCOResource>,
    pub meta: Meta,
}

#[derive(Debug, Deserialize, Clone)]
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

pub type IncludedHashes = (
    HashMap<String, Value>,
    HashMap<String, String>,
    HashMap<String, String>,
    HashMap<String, OrganizationInfo>,
    HashMap<String, HouseholdInfo>,
);

pub fn process_included(included: Vec<PCOResource>) -> IncludedHashes {
    let mut addresses: HashMap<String, Value> = HashMap::new();
    let mut emails: HashMap<String, String> = HashMap::new();
    let mut phones: HashMap<String, String> = HashMap::new();
    let mut organizations: HashMap<String, OrganizationInfo> = HashMap::new();
    let mut households: HashMap<String, HouseholdInfo> = HashMap::new();

    for item in included {
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
                        people: None,
                    },
                );
            }
            _ => {}
        }
    }

    (addresses, emails, phones, organizations, households)
}
