use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub host: String,
    pub port: u16,
    pub seed_demo_users: bool,
    pub demo_logins_public: bool,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".into());
        let is_production = app_env.eq_ignore_ascii_case("production");

        let seed_demo_users = env::var("SEED_DEMO_USERS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(!is_production);

        let demo_logins_public = env::var("DEMO_LOGINS_PUBLIC")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(!is_production);

        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:deploywerk.db?mode=rwc".into()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| {
                tracing::warn!("JWT_SECRET not set; using insecure dev default");
                "dev-insecure-change-me".into()
            }),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            seed_demo_users: seed_demo_users && !is_production,
            demo_logins_public: demo_logins_public && !is_production,
        }
    }
}
