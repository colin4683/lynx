# Lynx Documentation

This document provides information about Lynx, it will be split up into three main sections: lynx-core, lynx-agent, and
lynx-portal.
> **Overview:**\
> Lynx is a server monitoring solution. Several agents (lynx-agent) can be deployed on servers to collect system metrics
> and send them to the core (lynx-core) to be ingested into a Postgres database. The portal (lynx-portal) is a web
> application that connects to the database to visualize historical data and can also connect to agents via WebSocket
> for
> live data streaming.

## lynx-core

- Collects and stores metrics from agents in a Postgres database using gRPC
- Processes and manages notifications based on predefined rules made through the portal

### Security

- Uses TLS encryption for secure communication between agents and the core
    - TLS certificates can be self-signed or obtained from a trusted CA
    - Certificates are stored in `lynx-core/certs/`
    - Default paths:
        - `lynx-core/certs/server.crt`
        - `lynx-core/certs/server.key`
        - `lynx-core/certs/ca.crt`
- Authentication is handled using JWT tokens
    - Tokens are generated for each agent and must be included in the gRPC metadata for authentication
    - Tokens are stored in the database and can be managed through the portal
    - Tokens are generated when an agent is registered through the portal
    - Tokens can be revoked through the portal

## lynx-agent

- Deployed on servers to collect system metrics and send them to the core using gRPC
- Exposes a WebSocket connection for remote updates and command streams

### Security

- Uses TLS encryption for secure communication with the core
    - TLS certificates are stored in `lynx-agent/certs/`
    - CA certificate is obtained through the installation script
    - Default paths:
        - `lynx-agent/certs/agent.crt`
        - `lynx-agent/certs/agent.key`
        - `lynx-agent/certs/ca.crt`
        
