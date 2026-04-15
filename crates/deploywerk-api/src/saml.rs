//! Minimal SAML 2.0 ACS flow (experimental, insecure-by-default).
//!
//! This is a Phase 1 scaffold to unblock SAML-only environments.
//! It **does not** validate signatures yet unless an operator provides trusted enforcement externally.
//! Enable only when you control the IdP and transport, and treat as experimental.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::Engine as _;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{issue_token, require_principal};
use crate::error::ApiError;
use crate::rbac::user_summary_with_rbac;
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/saml/metadata", get(sp_metadata))
        .route("/api/v1/saml/acs", post(acs))
        .route(
            "/api/v1/organizations/{org_id}/saml/idps",
            get(list_idps).post(create_idp),
        )
}

async fn sp_metadata() -> Result<(StatusCode, String), ApiError> {
    // Minimal placeholder metadata. Real signing/certs are pending.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<EntityDescriptor xmlns="urn:oasis:names:tc:SAML:2.0:metadata" entityID="deploywerk-sp">
  <SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="/api/v1/saml/acs" index="0" isDefault="true"/>
  </SPSSODescriptor>
</EntityDescriptor>
"#;
    Ok((StatusCode::OK, xml.to_string()))
}

#[derive(Deserialize)]
struct CreateIdpBody {
    name: String,
    metadata_xml: String,
}

#[derive(Serialize)]
struct IdpRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    created_at: chrono::DateTime<Utc>,
}

async fn list_idps(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<IdpRow>>, ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_read()?;
    // For now: org mutators can view IdPs.
    crate::rbac::require_org_mutator(&state.pool, p.user_id, org_id).await?;

    let rows: Vec<(Uuid, Uuid, String, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, organization_id, name, created_at FROM saml_identity_providers WHERE organization_id = $1 ORDER BY created_at DESC",
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, organization_id, name, created_at)| IdpRow {
                id,
                organization_id,
                name,
                created_at,
            })
            .collect(),
    ))
}

async fn create_idp(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateIdpBody>,
) -> Result<(StatusCode, Json<IdpRow>), ApiError> {
    let p = require_principal(&state, &headers).await?;
    p.require_write()?;
    crate::rbac::require_org_mutator(&state.pool, p.user_id, org_id).await?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name required".into()));
    }
    let meta = body.metadata_xml.trim();
    if meta.is_empty() {
        return Err(ApiError::BadRequest("metadata_xml required".into()));
    }
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO saml_identity_providers (id, organization_id, name, metadata_xml, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(org_id)
    .bind(name)
    .bind(meta)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;

    Ok((
        StatusCode::CREATED,
        Json(IdpRow {
            id,
            organization_id: org_id,
            name: name.to_string(),
            created_at: now,
        }),
    ))
}

#[derive(Deserialize)]
struct SamlAcsForm {
    #[serde(rename = "SAMLResponse")]
    saml_response: String,
}

async fn acs(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SamlAcsForm>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let enabled = std::env::var("DEPLOYWERK_SAML_INSECURE_ENABLED")
        .ok()
        .map(|v| v == "true")
        .unwrap_or(false);
    if !enabled {
        return Err(ApiError::Forbidden);
    }

    // Decode base64 SAMLResponse and extract a usable email from NameID or common attributes.
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(form.saml_response.trim())
        .map_err(|_| ApiError::BadRequest("invalid SAMLResponse base64".into()))?;
    let xml = String::from_utf8(decoded).map_err(|_| ApiError::BadRequest("SAMLResponse not utf-8".into()))?;

    let doc = roxmltree::Document::parse(&xml).map_err(|_| ApiError::BadRequest("invalid SAML XML".into()))?;
    let email = doc
        .descendants()
        .find(|n| n.has_tag_name(("urn:oasis:names:tc:SAML:2.0:assertion", "NameID")))
        .and_then(|n| n.text())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && s.contains('@'))
        .ok_or_else(|| ApiError::BadRequest("missing NameID email".into()))?;

    // Find (any) org to attach via IdP metadata presence is not yet wired; for now, SAML creates a user only.
    // Operator should pre-provision org membership through SCIM or invites; this endpoint proves the auth path.
    let now = Utc::now();
    let user_id: Uuid = if let Some(id) = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE LOWER(email) = LOWER($1)")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?
    {
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, password_hash, name, created_at) VALUES ($1,$2,NULL,NULL,$3)")
            .bind(id)
            .bind(&email)
            .bind(now)
            .execute(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        id
    };

    let token = issue_token(user_id, &state.jwt_secret).map_err(|_| ApiError::Internal)?;
    let user = user_summary_with_rbac(
        &state.pool,
        deploywerk_core::UserSummary {
            id: user_id,
            email,
            name: None,
            current_team_id: None,
            current_organization_id: None,
            settings: Some(serde_json::json!({})),
            is_platform_admin: false,
            organization_admin_organization_ids: vec![],
            application_memberships: vec![],
        },
    )
    .await?;

    Ok(Json(serde_json::json!({ "token": token, "user": user })))
}

