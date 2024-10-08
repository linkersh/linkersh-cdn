use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use std::{env, fmt::Write as _};

use crate::db::PgClient;

pub fn generate_secret_code() -> String {
    let mut rng = rand::thread_rng();
    let mut hex_string = String::with_capacity(32);

    for _ in 0..16 {
        write!(&mut hex_string, "{:x}", rng.gen::<u8>()).unwrap();
    }

    hex_string
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenClaims {
    pub sub: Uuid,
    pub code: String,
    pub iss: String,
    pub exp: i64,
}

pub struct TokenHandler {
    enc_key: EncodingKey,
    dec_key: DecodingKey,
    header: Header,
    validation: Validation,
}

impl TokenHandler {
    // Preconstruct the keys, header, and validation
    pub fn new() -> anyhow::Result<Self> {
        let jwt_key = env::var("JWT_KEY")?;

        Ok(Self {
            enc_key: EncodingKey::from_base64_secret(&jwt_key)?,
            dec_key: DecodingKey::from_base64_secret(&jwt_key)?,
            header: Header::new(Algorithm::HS512),
            validation: Validation::new(Algorithm::HS512),
        })
    }

    pub async fn create_code(&self, client: &PgClient, user_id: Uuid) -> anyhow::Result<String> {
        let code = generate_secret_code();
        sqlx::query!(
            "INSERT INTO user_secret_codes (user_id, code) VALUES ($1, $2)",
            user_id,
            code
        )
        .execute(&client.inner)
        .await?;

        Ok(code)
    }

    pub fn sign_token(&self, sub: Uuid, code: String) -> anyhow::Result<String> {
        let claims = TokenClaims {
            sub,
            code,
            iss: String::from("https://linker.sh"),
            exp: Utc::now().timestamp_millis() + 60_000 * 60 * 24,
        };

        let token = jsonwebtoken::encode(&self.header, &claims, &self.enc_key)?;
        Ok(token)
    }

    pub fn verify_token(&self, token: &str) -> anyhow::Result<TokenClaims> {
        let token_data: TokenData<TokenClaims> =
            jsonwebtoken::decode(token, &self.dec_key, &self.validation)?;
        Ok(token_data.claims)
    }
}
