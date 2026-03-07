use axum::extract::{Json, State};
use axum::response::IntoResponse;
use serde::Serialize;

use super::ApiResult;
use crate::profile::{CreateProfileRequest, Profile};
use crate::state::AppState;

#[derive(Serialize)]
pub(crate) struct ProfilesResponse {
    profiles: Vec<Profile>,
}

#[derive(Serialize)]
pub(crate) struct ProfileResponse {
    profile: Profile,
}

pub async fn list_profiles(State(state): State<AppState>) -> impl IntoResponse {
    let profiles = state.profiles.read().await;
    let list: Vec<Profile> = profiles.values().cloned().collect();
    Json(ProfilesResponse { profiles: list })
}

pub async fn create_profile(
    State(state): State<AppState>,
    Json(req): Json<CreateProfileRequest>,
) -> ApiResult<ProfileResponse> {
    let profile = Profile::new(&req.name)
        .map_err(|e| super::api_err(axum::http::StatusCode::BAD_REQUEST, &e))?;
    let resp = ProfileResponse { profile: profile.clone() };
    state
        .profiles
        .write()
        .await
        .insert(profile.name.clone(), profile);
    Ok(Json(resp))
}
