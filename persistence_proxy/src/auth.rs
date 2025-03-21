use crate::AppState;
use crate::db::consumer::Column::TokenHash;
use crate::db::prelude::Consumer;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub struct AuthExtractor {
    pub consumer_id: i32,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthExtractor
where
    AppState: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.split(" ").nth(1))
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let hashed_token = blake3::hash(token.as_bytes()).to_hex().to_string();

        let state_ref = AppState::from_ref(state);

        let participant = Consumer::find()
            .filter(TokenHash.contains(hashed_token))
            .one(&state_ref.db)
            .await
            .ok()
            .flatten()
            .ok_or(StatusCode::UNAUTHORIZED)?;
        Ok(AuthExtractor {
            consumer_id: participant.id,
        })
    }
}
