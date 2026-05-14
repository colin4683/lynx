#!/bin/bash

# Start lynx-agent in debug mode
echo "Starting lynx-agent..."
cargo run --manifest-path lynx-agent/Cargo.toml &

AGENT_PID=$!

# Start lynx-core in debug mode
echo "Starting lynx-core..."
cargo run --manifest-path lynx-core/Cargo.toml &

CORE_PID=$!

echo "lynx-agent PID: $AGENT_PID"
echo "lynx-core PID: $CORE_PID"

# Wait for both to finish
wait $AGENT_PID
wait $CORE_PID

echo "Both lynx-agent and lynx-core have exited."