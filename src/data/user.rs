use crate::data::steam_id::SteamId;
use crate::{Error, Result};
use rand::distributions::Alphanumeric;
use rand::Rng;
use reqwest::get;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Executor, Postgres};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub steam_id: SteamId,
    pub name: String,
    pub token: String,
}

struct UserResult {
    name: String,
    token: String,
}

impl User {
    pub async fn get(
        connection: impl Executor<'_, Database = Postgres> + Copy,
        steam_id: SteamId,
    ) -> Result<Self> {
        let mut user = query_as!(
            UserResult,
            r#"SELECT
                token as "token!", name as "name!"
            FROM users_named WHERE steamid = $1"#,
            steam_id.steamid64()
        )
        .fetch_optional(connection)
        .await?;

        if user.is_none() {
            user = query_as!(
                UserResult,
                r#"SELECT
                    token as "token!", name as "name!"
                FROM users WHERE steamid = $1"#,
                steam_id.steamid64()
            )
            .fetch_optional(connection)
            .await?;
        }

        if let Some(user) = user {
            Ok(User {
                steam_id,
                token: user.token,
                name: user.name,
            })
        } else {
            let profile = Self::fetch(&steam_id).await?;
            let token: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();

            query!(
                r#"INSERT INTO users(steamid, name, avatar, token)
                    VALUES($1, $2, $3, $4)"#,
                steam_id.steamid64(),
                profile.name,
                profile.avatar,
                token
            )
            .execute(connection)
            .await?;
            Ok(User {
                steam_id,
                token,
                name: profile.name,
            })
        }
    }

    async fn fetch(steam_id: &SteamId) -> Result<Profile> {
        let SteamId::Id(steam_id) = steam_id else {
            return Err(Error::NotFound);
        };
        let response = get(format!(
            "https://steamcommunity.com/profiles/{steam_id}?xml=1"
        ))
        .await?
        .error_for_status()?
        .text()
        .await?;
        Ok(quick_xml::de::from_str(&response)?)
    }
}

#[derive(Debug, Deserialize)]
struct Profile {
    #[serde(rename = "steamID")]
    name: String,
    #[serde(rename = "avatarMedium")]
    avatar: String,
}
