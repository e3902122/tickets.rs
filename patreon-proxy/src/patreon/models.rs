use chrono::{DateTime, Utc};
use serde::Deserialize;

use std::collections::HashMap;

use super::Entitlement;

#[derive(Debug, Deserialize)]
pub struct PledgeResponse {
    pub data: Vec<Member>,
    pub included: Vec<PatronMetadata>,
    pub links: Option<Links>,
}

#[derive(Debug, Deserialize)]
pub struct Member {
    pub attributes: MemberAttributes,
    pub relationships: Relationships,
}

#[derive(Debug, Deserialize)]
pub struct MemberAttributes {
    pub last_charge_date: Option<DateTime<Utc>>,
    pub last_charge_status: Option<ChargeStatus>,
    pub next_charge_date: Option<DateTime<Utc>>,
    pub pledge_cadence: Option<u16>,
    pub patron_status: Option<PatronStatus>, // null = never pledged
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ChargeStatus {
    Paid,
    Declined,
    Deleted,
    Pending,
    Refunded,
    Fraud,
    Other,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PatronStatus {
    ActivePatron,
    FormerPatron,
    DeclinedPatron,
}

#[derive(Debug, Deserialize)]
pub struct Relationships {
    pub user: User,
    pub currently_entitled_tiers: EntitledTiers,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub data: UserData,
}

#[derive(Debug, Deserialize)]
pub struct UserData {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct EntitledTiers {
    pub data: Vec<EntitledTier>,
}

#[derive(Debug, Deserialize)]
pub struct EntitledTier {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct PatronMetadata {
    pub id: String,
    pub attributes: PatronAttributes,
}

#[derive(Debug, Deserialize)]
pub struct PatronAttributes {
    pub social_connections: Option<SocialConnections>,
}

#[derive(Debug, Deserialize)]
pub struct SocialConnections {
    pub discord: Option<DiscordConnection>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordConnection {
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Links {
    pub next: Option<String>,
}

impl PledgeResponse {
    // returns user ID -> tier
    pub fn convert(&self) -> HashMap<String, Vec<Entitlement>> {
        self.data
            .iter()
            .filter(|member| {
                member.attributes.patron_status == Some(PatronStatus::ActivePatron)
                    && (member.attributes.last_charge_status == Some(ChargeStatus::Paid)
                        || member.attributes.last_charge_status == Some(ChargeStatus::Pending))
                    && !member
                        .relationships
                        .currently_entitled_tiers
                        .data
                        .is_empty() // Make sure the user is subscribed to a tier
            })
            .filter_map(|member| -> Option<(String, Vec<super::Entitlement>)> {
                let meta = self.get_meta_by_id(member.relationships.user.data.id.as_str())?;

                // Find all subscribed tiers (is this possible?)
                let entitlements = member
                    .relationships
                    .currently_entitled_tiers
                    .data
                    .iter()
                    .map(|tier| Entitlement::entitled_skus(tier.id.as_str(), &member.attributes))
                    .flatten()
                    .collect();

                let discord_id = meta
                    .attributes
                    .social_connections
                    .as_ref()
                    .and_then(|sc| sc.discord.as_ref())
                    .map(|d| d.user_id.clone())
                    .flatten()?;
                
                Some((discord_id, entitlements))
            })
            .filter(|(_, skus)| !skus.is_empty())
            .collect()
    }

    fn get_meta_by_id(&self, id: &str) -> Option<&PatronMetadata> {
        self.included.iter().find(|metadata| metadata.id == id)
    }
}
