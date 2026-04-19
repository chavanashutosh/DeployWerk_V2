//! SQL fragments that differ between PostgreSQL and SQLite.

#[cfg(feature = "postgres")]
pub fn coalesce_user_prefs_settings_json() -> &'static str {
    "COALESCE(settings_json, '{}'::jsonb)"
}

#[cfg(feature = "sqlite")]
pub fn coalesce_user_prefs_settings_json() -> &'static str {
    "COALESCE(settings_json, '{}')"
}

#[cfg(feature = "postgres")]
pub fn coalesce_up_settings_json() -> &'static str {
    "COALESCE(up.settings_json, '{}'::jsonb)"
}

#[cfg(feature = "sqlite")]
pub fn coalesce_up_settings_json() -> &'static str {
    "COALESCE(up.settings_json, '{}')"
}

#[cfg(feature = "postgres")]
pub fn coalesce_jsonb_empty_array() -> &'static str {
    "'[]'::jsonb"
}

#[cfg(feature = "sqlite")]
pub fn coalesce_jsonb_empty_array() -> &'static str {
    "'[]'"
}

#[cfg(feature = "postgres")]
pub fn insert_user_prefs_empty_settings() -> &'static str {
    "INSERT INTO user_preferences (user_id, settings_json) VALUES ($1, '{}'::jsonb) ON CONFLICT (user_id) DO NOTHING"
}

#[cfg(feature = "sqlite")]
pub fn insert_user_prefs_empty_settings() -> &'static str {
    "INSERT INTO user_preferences (user_id, settings_json) VALUES ($1, '{}') ON CONFLICT (user_id) DO NOTHING"
}
