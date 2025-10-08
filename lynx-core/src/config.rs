use env_logger::Env;
use log::info;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub retention_days: i64,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable is not set")?;
        let retention_days = std::env::var("RETENTION_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<i64>()
            .unwrap_or(30);
        Ok(Self {
            database_url,
            retention_days,
        })
    }
}

pub fn load_env() {
    dotenv::dotenv().ok();
}

pub fn init_logging() {
    let env = Env::default()
        .filter("MY_LOG_LEVEL")
        .write_style("MY_LOG_STYLE");
    env_logger::Builder::from_env(env)
        .format_timestamp_secs()
        .init();
    info!("[hub] Logging initialized");
}
