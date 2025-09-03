use uuid::Uuid;

/// Generate an installation script for an inactive (pending) agent.
/// Activates the agent (sets key + active=true) if hostname + token match.
pub async fn generate_agent_install_script(
    hostname: &str,
    token: &str,
    pool: &sqlx::PgPool,
) -> Result<String, Box<dyn std::error::Error>> {
    let agent = sqlx::query!(
        r"SELECT id FROM systems WHERE hostname = $1 AND token = $2 AND active = false",
        hostname,
        token
    )
    .fetch_optional(pool)
    .await?;

    if agent.is_none() {
        return Err("Invalid hostname or token".into());
    }

    let agent_key = Uuid::new_v4().to_string();

    let script = format!(
        r##"#!/bin/bash
# Auto-generated install script for Lynx Agent

set -euo pipefail

BIN_URL="https://example.com/agent/lynx-agent"
INSTALL_PATH="/usr/local/bin/lynx-view-agent"
CONFIG_DIR="/etc/lynx-view"
SERVICE_FILE="/etc/systemd/system/lynx-view-agent.service"

curl -fsSL "$BIN_URL" -o "$INSTALL_PATH"
chmod +x "$INSTALL_PATH"

mkdir -p "$CONFIG_DIR"
cat > "$CONFIG_DIR/config.toml" <<EOF
[core]
server_url = "grpc://localhost:50051"
agent_key = "{agent_key}"
EOF

cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Lynx Agent
After=network-online.target

[Service]
ExecStart=$INSTALL_PATH
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now lynx-view-agent
"##
    );

    sqlx::query!(
        r"UPDATE systems SET active = true, key = $1 WHERE id = $2",
        agent_key,
        agent.unwrap().id
    )
    .execute(pool)
    .await?;

    Ok(script)
}
