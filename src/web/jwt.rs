use chrono;
use jsonwebtoken::crypto::verify;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};

use crate::core::db::QueryArg;

#[derive(Deserialize, Serialize, Debug)]
pub struct RequestQuery<'a> {
    pub base_query: String,
    #[serde(borrow)]
    pub parts: Vec<QueryArg<'a>>,
}

#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub enum Iss {
    S_,
    C_,
}

#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub enum Sub {
    M_,
    F_,
    D_,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub iss: Iss,
    pub sub: Sub,
    // pub aud: String,
    pub dat: String,
    pub iat: usize,
    pub exp: usize,
}

pub fn generate_claims(content: String, subject: Sub) -> Claims {
    let claims = Claims {
        iss: Iss::S_,
        sub: subject,
        // aud: String::from("c_"),
        dat: content,
        iat: chrono::Utc::now().timestamp() as usize,
        exp: (chrono::Utc::now() + chrono::Duration::seconds(30)).timestamp() as usize,
    };

    claims
}

pub fn generate_token(claims: Claims, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<bool, jsonwebtoken::errors::Error> {
    let token_parts: Vec<&str> = token.split(".").collect();

    let head = token_parts[0];
    let claim = token_parts[1];
    let sig = token_parts[2];

    verify(
        sig,
        format!("{}.{}", head, claim).as_bytes(),
        &DecodingKey::from_secret(secret.as_bytes()),
        Algorithm::HS256,
    )
}

pub fn decode_token(
    token: &str,
    secret: &str,
) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    let validation = Validation::new(Algorithm::HS256);
    // validation.set_audience(&["c_"]); // may need to use this at some point

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
}
