use crate::{commands::AppState, storage::IntegrationClient, storage::StorageError};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderMap, StatusCode},
};

pub const ORIGIN_HEADER: &str = "x-ankicode-origin";

#[derive(Debug, Clone)]
pub struct AuthenticatedClient(pub IntegrationClient);

pub fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_owned())
    }
}

pub fn extract_request_origin(headers: &HeaderMap) -> Option<String> {
    if let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        let trimmed = origin.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }
    headers
        .get(ORIGIN_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub fn auth_status(error: &StorageError) -> StatusCode {
    match error {
        StorageError::UnauthorizedClient | StorageError::InvalidPairingCode => {
            StatusCode::UNAUTHORIZED
        }
        StorageError::ClientRevoked
        | StorageError::OriginMismatch
        | StorageError::InvalidOrigin => StatusCode::FORBIDDEN,
        StorageError::ProblemNotFound(_) | StorageError::ProblemSlugNotFound(_) => {
            StatusCode::NOT_FOUND
        }
        StorageError::IntegrationIdempotencyConflict { .. }
        | StorageError::ReviewIdempotencyConflict { .. } => StatusCode::CONFLICT,
        StorageError::InvalidData(_) | StorageError::Problem(_) | StorageError::Settings(_) => {
            StatusCode::BAD_REQUEST
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl FromRequestParts<AppState> for AuthenticatedClient {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer_token(&parts.headers)
            .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".to_owned()))?;
        let origin = extract_request_origin(&parts.headers)
            .ok_or((StatusCode::FORBIDDEN, "missing origin".to_owned()))?;
        let guard = state.inner.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "lock poisoned".to_owned(),
            )
        })?;
        let client = guard
            .db
            .authenticate_client(&token, &origin)
            .map_err(|error| (auth_status(&error), error.to_string()))?;
        Ok(Self(client))
    }
}
