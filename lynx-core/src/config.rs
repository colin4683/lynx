use env_logger::Env;
use log::info;

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

pub fn database_url() -> Result<String, std::env::VarError> {
    std::env::var("DATABASE_URL")
}
