pub mod alert_queries {
    pub const GET_ALERT_SYSTEMS: &str = "SELECT rule_id FROM alert_systems WHERE system_id = $1";

    pub const GET_ALERT_RULES: &str = "SELECT id, name, description, active, expression, severity FROM alert_rules WHERE id = $1 AND active = true";

    pub const GET_ALERT_NOTIFIERS: &str =
        "SELECT rule_id, notifier_id FROM alert_notifiers WHERE rule_id = $1";

    pub const GET_NOTIFIERS: &str = "SELECT id, type, value FROM notifiers WHERE id = $1";

    pub const GET_EXISTING_ALERT: &str = "SELECT id FROM alert_history WHERE system = $1 AND alert = $2 AND date >= NOW() - INTERVAL '30 minutes'";

    pub const UPDATE_ALERT_HISTORY: &str = "UPDATE alert_history SET date = NOW() WHERE id = $1";

    pub const INSERT_ALERT_HISTORY: &str =
        "INSERT INTO alert_history (system, alert, date) VALUES ($1, $2, NOW())";
}
