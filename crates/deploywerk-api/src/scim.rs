//! Inbound SCIM 2.0 for Authentik (or any client) to provision users and map group memberships to RBAC.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header::AUTHORIZATION, HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use deploywerk_core::{AppRole, TeamRole};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::auth::parse_bearer_token;
use crate::error::ApiError;
use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/scim/v2/ServiceProviderConfig", get(service_provider_config))
        .route("/scim/v2/Schemas", get(schemas))
        .route("/scim/v2/Users", get(list_users).post(post_user))
        .route(
            "/scim/v2/Users/{id}",
            get(get_user)
                .put(put_user)
                .patch(patch_user)
                .delete(delete_user),
        )
        .route("/scim/v2/Groups", get(list_groups).post(post_group))
        .route(
            "/scim/v2/Groups/{id}",
            get(get_group)
                .patch(patch_group)
                .delete(delete_group),
        )
}

fn require_scim(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let token = state.scim_bearer_token.as_deref().ok_or(ApiError::NotFound)?;
    let auth = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());
    let got = parse_bearer_token(auth).ok_or(ApiError::Unauthorized)?;
    if got.len() != token.len() || got.as_bytes().ct_eq(token.as_bytes()).unwrap_u8() != 1 {
        return Err(ApiError::Unauthorized);
    }
    Ok(())
}

// --- ServiceProviderConfig / Schemas ---

async fn service_provider_config(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    Ok(Json(json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
        "patch": { "supported": true },
        "bulk": { "supported": false },
        "filter": { "supported": true, "maxResults": 200 },
        "changePassword": { "supported": false },
        "sort": { "supported": false },
        "etag": { "supported": false },
        "authenticationSchemes": []
    })))
}

async fn schemas(headers: HeaderMap, State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    Ok(Json(json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": 0,
        "Resources": []
    })))
}

// --- Users ---

#[derive(Deserialize)]
struct ScimListQuery {
    filter: Option<String>,
}

async fn list_users(
    Query(q): Query<ScimListQuery>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    let pool = &state.pool;
    if let Some(ref f) = q.filter {
        if let Some(ext) = parse_filter_external_id(f) {
            let row: Option<(Uuid, String, Option<String>, Option<String>, Option<String>)> =
                sqlx::query_as(
                    "SELECT id, email, name, idp_issuer, idp_subject FROM users WHERE idp_subject = $1",
                )
                .bind(&ext)
                .fetch_optional(pool)
                .await
                .map_err(|_| ApiError::Internal)?;
            if let Some(u) = row {
                return Ok(Json(list_response(vec![user_resource(
                    u.0, &u.1, u.2.as_deref(), u.3.as_deref(), u.4.as_deref(),
                )])));
            }
        }
    }
    Ok(Json(list_response(vec![])))
}

fn parse_filter_external_id(filter: &str) -> Option<String> {
    let t = filter.trim();
    // externalId eq "value" or externalId eq \"value\"
    let rest = t.strip_prefix("externalId")?.trim_start().strip_prefix("eq")?.trim();
    let rest = rest.trim();
    let s = rest
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .or_else(|| rest.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))?;
    Some(s.to_string())
}

fn list_response(resources: Vec<Value>) -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": resources.len(),
        "itemsPerPage": resources.len(),
        "startIndex": 1,
        "Resources": resources
    })
}

fn user_resource(
    id: Uuid,
    email: &str,
    name: Option<&str>,
    idp_issuer: Option<&str>,
    idp_subject: Option<&str>,
) -> Value {
    let mut v = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": id.to_string(),
        "userName": email,
        "active": true,
        "emails": [{"value": email, "primary": true, "type": "work"}]
    });
    if let Some(n) = name {
        v["displayName"] = json!(n);
    }
    if let Some(sub) = idp_subject {
        v["externalId"] = json!(sub);
    } else if let Some(iss) = idp_issuer {
        v["externalId"] = json!(format!("{iss}#{}", id));
    }
    v
}

async fn get_user(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    let row: Option<(String, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT email, name, idp_issuer, idp_subject FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    let Some((email, name, iss, sub)) = row else {
        return Err(ApiError::NotFound);
    };
    Ok(Json(user_resource(
        id,
        &email,
        name.as_deref(),
        iss.as_deref(),
        sub.as_deref(),
    )))
}

fn scim_primary_email(v: &Value) -> Option<String> {
    if let Some(s) = v.get("userName").and_then(|x| x.as_str()) {
        let e = s.trim().to_lowercase();
        if e.contains('@') {
            return Some(e);
        }
    }
    v.get("emails")?
        .as_array()?
        .iter()
        .find(|e| e.get("primary").and_then(|p| p.as_bool()).unwrap_or(false))
        .or_else(|| v.get("emails")?.as_array()?.first())
        .and_then(|e| e.get("value"))
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_lowercase())
        .filter(|e| e.contains('@'))
}

fn scim_external_id(v: &Value) -> Option<String> {
    v.get("externalId")
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn scim_display_name(v: &Value) -> Option<String> {
    v.get("displayName")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            v.pointer("/name/formatted")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        })
}

async fn upsert_user_from_scim(
    pool: &PgPool,
    idp_issuer: &str,
    body: &Value,
    fixed_id: Option<Uuid>,
) -> Result<Uuid, ApiError> {
    let email = scim_primary_email(body).ok_or_else(|| {
        ApiError::BadRequest("SCIM user requires userName or emails[].value".into())
    })?;
    let ext = scim_external_id(body);
    let name = scim_display_name(body);

    if let Some(id) = fixed_id {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
        if exists == 0 {
            let sub = ext.clone().ok_or_else(|| {
                ApiError::BadRequest("SCIM user PATCH: unknown id; provide externalId".into())
            })?;
            let now = Utc::now();
            sqlx::query(
                "INSERT INTO users (id, email, password_hash, name, created_at, idp_issuer, idp_subject) VALUES ($1, $2, NULL, $3, $4, $5, $6)",
            )
            .bind(id)
            .bind(&email)
            .bind(&name)
            .bind(now)
            .bind(idp_issuer)
            .bind(&sub)
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            sqlx::query(
                "INSERT INTO user_preferences (user_id, settings_json) VALUES ($1, '{}'::jsonb) ON CONFLICT (user_id) DO NOTHING",
            )
            .bind(id)
            .execute(pool)
            .await
            .map_err(|_| ApiError::Internal)?;
            return Ok(id);
        }
        sqlx::query(
            "UPDATE users SET email = $1, name = COALESCE($2, name), idp_issuer = COALESCE($3, idp_issuer), idp_subject = COALESCE($4, idp_subject) WHERE id = $5",
        )
        .bind(&email)
        .bind(&name)
        .bind(idp_issuer)
        .bind(&ext)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        return Ok(id);
    }

    if let Some(ref sub) = ext {
        let by_sub: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM users WHERE idp_issuer = $1 AND idp_subject = $2",
        )
        .bind(idp_issuer)
        .bind(sub)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        if let Some(id) = by_sub {
            sqlx::query("UPDATE users SET email = $1, name = COALESCE($2, name) WHERE id = $3")
                .bind(&email)
                .bind(&name)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|_| ApiError::Internal)?;
            return Ok(id);
        }
    }

    let by_email: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE LOWER(email) = LOWER($1)")
        .bind(&email)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    if let Some(id) = by_email {
        sqlx::query(
            "UPDATE users SET idp_issuer = $1, idp_subject = COALESCE($2, idp_subject), name = COALESCE($3, name) WHERE id = $4",
        )
        .bind(idp_issuer)
        .bind(&ext)
        .bind(&name)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
        return Ok(id);
    }

    let id = Uuid::new_v4();
    let sub = ext.ok_or_else(|| ApiError::BadRequest("SCIM user requires externalId".into()))?;
    let now = Utc::now();
    let mut tx = pool.begin().await.map_err(|_| ApiError::Internal)?;
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, created_at, idp_issuer, idp_subject) VALUES ($1, $2, NULL, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(&email)
    .bind(&name)
    .bind(now)
    .bind(idp_issuer)
    .bind(&sub)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;
    sqlx::query(
        "INSERT INTO user_preferences (user_id, settings_json) VALUES ($1, '{}'::jsonb) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal)?;
    tx.commit().await.map_err(|_| ApiError::Internal)?;
    Ok(id)
}

async fn post_user(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    require_scim(&state, &headers)?;
    let idp = state
        .scim_idp_issuer
        .as_deref()
        .ok_or_else(|| ApiError::Internal)?;
    let id = upsert_user_from_scim(&state.pool, idp, &body, None).await?;
    let row: (String, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT email, name, idp_issuer, idp_subject FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Internal)?;
    Ok((
        StatusCode::CREATED,
        Json(user_resource(
            id,
            &row.0,
            row.1.as_deref(),
            row.2.as_deref(),
            row.3.as_deref(),
        )),
    ))
}

async fn put_user(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    let idp = state
        .scim_idp_issuer
        .as_deref()
        .ok_or_else(|| ApiError::Internal)?;
    upsert_user_from_scim(&state.pool, idp, &body, Some(id)).await?;
    get_user(Path(id), headers, State(state)).await
}

async fn patch_user(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    let v: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    if let Some(ops) = v.get("Operations").and_then(|o| o.as_array()) {
        let mut merged = json!({});
        for op in ops {
            let op_name = op.get("op").and_then(|x| x.as_str()).unwrap_or("").to_lowercase();
            if op_name != "replace" && op_name != "add" {
                continue;
            }
            let path = op.get("path").and_then(|x| x.as_str()).unwrap_or("");
            if path.is_empty() {
                if let Some(val) = op.get("value") {
                    if let Some(obj) = val.as_object() {
                        merged = serde_json::Value::Object(obj.clone());
                    }
                }
            }
        }
        if merged.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
            let idp = state
                .scim_idp_issuer
                .as_deref()
                .ok_or_else(|| ApiError::Internal)?;
            upsert_user_from_scim(&state.pool, idp, &merged, Some(id)).await?;
        }
    }
    get_user(Path(id), headers, State(state)).await
}

async fn delete_user(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    require_scim(&state, &headers)?;
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Groups ---

#[derive(Debug, Clone)]
enum GroupBinding {
    PlatformAdmin,
    Org { id: Uuid, role: TeamRole },
    Team { id: Uuid, role: TeamRole },
    App { id: Uuid, role: AppRole },
}

fn parse_group_display_name(s: &str) -> Option<GroupBinding> {
    let s = s.trim();
    if s == "deploywerk-platform-admin" {
        return Some(GroupBinding::PlatformAdmin);
    }
    if let Some(rest) = s.strip_prefix("deploywerk-org-") {
        for r in ["owner", "admin", "member"] {
            if let Some(uuid_part) = rest.strip_suffix(&format!("-{r}")) {
                if let Ok(id) = Uuid::parse_str(uuid_part) {
                    return TeamRole::parse(r).map(|role| GroupBinding::Org { id, role });
                }
            }
        }
        return None;
    }
    if let Some(rest) = s.strip_prefix("deploywerk-team-") {
        for r in ["owner", "admin", "member"] {
            if let Some(uuid_part) = rest.strip_suffix(&format!("-{r}")) {
                if let Ok(id) = Uuid::parse_str(uuid_part) {
                    return TeamRole::parse(r).map(|role| GroupBinding::Team { id, role });
                }
            }
        }
    }
    if let Some(rest) = s.strip_prefix("deploywerk-app-") {
        for r in ["admin", "viewer"] {
            if let Some(uuid_part) = rest.strip_suffix(&format!("-{r}")) {
                if let Ok(id) = Uuid::parse_str(uuid_part) {
                    if let Some(role) = AppRole::parse(r) {
                        return Some(GroupBinding::App { id, role });
                    }
                }
            }
        }
    }
    None
}

async fn apply_group_membership(
    pool: &PgPool,
    user_id: Uuid,
    display_name: &str,
    add: bool,
) -> Result<(), ApiError> {
    let Some(binding) = parse_group_display_name(display_name) else {
        tracing::warn!(display_name, "SCIM group displayName not recognized; ignored");
        return Ok(());
    };
    match binding {
        GroupBinding::PlatformAdmin => {
            sqlx::query("UPDATE users SET is_platform_admin = $1 WHERE id = $2")
                .bind(add)
                .bind(user_id)
                .execute(pool)
                .await
                .map_err(|_| ApiError::Internal)?;
        }
        GroupBinding::Org { id, role } => {
            if add {
                sqlx::query(
                    r#"INSERT INTO organization_memberships (user_id, organization_id, role) VALUES ($1, $2, $3)
                       ON CONFLICT (user_id, organization_id) DO UPDATE SET role = EXCLUDED.role"#,
                )
                .bind(user_id)
                .bind(id)
                .bind(role.as_str())
                .execute(pool)
                .await
                .map_err(|_| ApiError::Internal)?;
            } else {
                sqlx::query(
                    "DELETE FROM organization_memberships WHERE user_id = $1 AND organization_id = $2",
                )
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|_| ApiError::Internal)?;
            }
        }
        GroupBinding::Team { id, role } => {
            if add {
                sqlx::query(
                    r#"INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, $3)
                       ON CONFLICT (user_id, team_id) DO UPDATE SET role = EXCLUDED.role"#,
                )
                .bind(user_id)
                .bind(id)
                .bind(role.as_str())
                .execute(pool)
                .await
                .map_err(|_| ApiError::Internal)?;
            } else {
                sqlx::query("DELETE FROM team_memberships WHERE user_id = $1 AND team_id = $2")
                    .bind(user_id)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|_| ApiError::Internal)?;
            }
        }
        GroupBinding::App { id, role } => {
            if add {
                sqlx::query(
                    r#"INSERT INTO application_memberships (user_id, application_id, role) VALUES ($1, $2, $3)
                       ON CONFLICT (user_id, application_id) DO UPDATE SET role = EXCLUDED.role"#,
                )
                .bind(user_id)
                .bind(id)
                .bind(role.as_str())
                .execute(pool)
                .await
                .map_err(|_| ApiError::Internal)?;
            } else {
                sqlx::query(
                    "DELETE FROM application_memberships WHERE user_id = $1 AND application_id = $2",
                )
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|_| ApiError::Internal)?;
            }
        }
    }
    Ok(())
}

fn group_resource(id: Uuid, display_name: &str, members: Vec<Value>) -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": id.to_string(),
        "displayName": display_name,
        "members": members
    })
}

async fn load_group_display_name(pool: &PgPool, id: Uuid) -> Result<String, ApiError> {
    let name: Option<String> = sqlx::query_scalar("SELECT display_name FROM scim_groups WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    name.ok_or(ApiError::NotFound)
}

async fn list_group_members(pool: &PgPool, group_id: Uuid) -> Result<Vec<Value>, ApiError> {
    let display_name = load_group_display_name(pool, group_id).await?;
    let Some(binding) = parse_group_display_name(&display_name) else {
        return Ok(vec![]);
    };
    let rows: Vec<(Uuid,)> = match binding {
        GroupBinding::PlatformAdmin => sqlx::query_as("SELECT id FROM users WHERE is_platform_admin = TRUE")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::Internal)?,
        GroupBinding::Org { id, role } => sqlx::query_as(
            "SELECT user_id FROM organization_memberships WHERE organization_id = $1 AND role = $2",
        )
        .bind(id)
        .bind(role.as_str())
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::Internal)?,
        GroupBinding::Team { id, role } => sqlx::query_as(
            "SELECT user_id FROM team_memberships WHERE team_id = $1 AND role = $2",
        )
        .bind(id)
        .bind(role.as_str())
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::Internal)?,
        GroupBinding::App { id, role } => sqlx::query_as(
            "SELECT user_id FROM application_memberships WHERE application_id = $1 AND role = $2",
        )
        .bind(id)
        .bind(role.as_str())
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::Internal)?,
    };
    Ok(rows
        .into_iter()
        .map(|(uid,)| json!({ "value": uid.to_string(), "$ref": format!("/scim/v2/Users/{}", uid) }))
        .collect())
}

async fn list_groups(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    let rows: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, display_name FROM scim_groups ORDER BY display_name")
            .fetch_all(&state.pool)
            .await
            .map_err(|_| ApiError::Internal)?;
    let mut resources = Vec::with_capacity(rows.len());
    for (id, display_name) in rows {
        let members = list_group_members(&state.pool, id).await?;
        resources.push(group_resource(id, &display_name, members));
    }
    Ok(Json(list_response(resources)))
}

async fn post_group(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    require_scim(&state, &headers)?;
    let display = body
        .get("displayName")
        .and_then(|x| x.as_str())
        .ok_or_else(|| ApiError::BadRequest("SCIM group requires displayName".into()))?
        .to_string();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO scim_groups (id, display_name) VALUES ($1, $2)")
        .bind(id)
        .bind(&display)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(d) = &e {
                if d.code().as_deref() == Some("23505") {
                    return ApiError::Conflict("group displayName already exists".into());
                }
            }
            ApiError::Internal
        })?;
    if let Some(m) = body.get("members").and_then(|x| x.as_array()) {
        for mem in m {
            let uid_str = mem
                .get("value")
                .and_then(|x| x.as_str())
                .ok_or_else(|| ApiError::BadRequest("SCIM group member requires value".into()))?;
            let uid = Uuid::parse_str(uid_str.trim()).map_err(|_| ApiError::BadRequest("invalid member id".into()))?;
            apply_group_membership(&state.pool, uid, &display, true).await?;
        }
    }
    let members = list_group_members(&state.pool, id).await?;
    Ok((StatusCode::CREATED, Json(group_resource(id, &display, members))))
}

async fn get_group(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    let display = load_group_display_name(&state.pool, id).await?;
    let members = list_group_members(&state.pool, id).await?;
    Ok(Json(group_resource(id, &display, members)))
}

async fn patch_group(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    require_scim(&state, &headers)?;
    let display = load_group_display_name(&state.pool, id).await?;
    let Some(ops) = body.get("Operations").and_then(|o| o.as_array()) else {
        return get_group(Path(id), headers, State(state)).await;
    };
    for op in ops {
        let op_name = op.get("op").and_then(|x| x.as_str()).unwrap_or("").to_lowercase();
        let path = op.get("path").and_then(|x| x.as_str()).unwrap_or("");
        if path == "members" || path.starts_with("members[") {
            if op_name == "add" {
                let vals = op
                    .get("value")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                for mem in vals {
                    let uid_str = mem
                        .get("value")
                        .and_then(|x| x.as_str())
                        .ok_or_else(|| ApiError::BadRequest("member value required".into()))?;
                    let uid =
                        Uuid::parse_str(uid_str.trim()).map_err(|_| ApiError::BadRequest("invalid member".into()))?;
                    apply_group_membership(&state.pool, uid, &display, true).await?;
                }
            } else if op_name == "remove" {
                if let Some(uid_str) = path.strip_prefix("members[value eq \"").and_then(|s| s.strip_suffix("\"]")) {
                    let uid = Uuid::parse_str(uid_str.trim())
                        .map_err(|_| ApiError::BadRequest("invalid member path".into()))?;
                    apply_group_membership(&state.pool, uid, &display, false).await?;
                }
            }
        }
    }
    get_group(Path(id), headers, State(state)).await
}

async fn delete_group(
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    require_scim(&state, &headers)?;
    let display = load_group_display_name(&state.pool, id).await?;
    let members = list_group_members(&state.pool, id).await?;
    for m in members {
        if let Some(s) = m.get("value").and_then(|x| x.as_str()) {
            if let Ok(uid) = Uuid::parse_str(s) {
                let _ = apply_group_membership(&state.pool, uid, &display, false).await;
            }
        }
    }
    sqlx::query("DELETE FROM scim_groups WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}
