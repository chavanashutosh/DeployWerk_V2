//! Database pool type: exactly one of `postgres` or `sqlite` Cargo feature must be enabled.

#[cfg(all(feature = "postgres", feature = "sqlite"))]
compile_error!("Enable exactly one database feature: `postgres` or `sqlite`");

#[cfg(all(not(feature = "postgres"), not(feature = "sqlite")))]
compile_error!("Enable exactly one database feature: `postgres` or `sqlite`");

#[cfg(feature = "postgres")]
pub type DbPool = sqlx::PgPool;

#[cfg(feature = "sqlite")]
pub type DbPool = sqlx::SqlitePool;
