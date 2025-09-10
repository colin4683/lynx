# Lynx

A lightweight and beautiful server monitoring service.

Comes with a sleek and user friendly web interface allowing you to deploy agents, setup alerts, and view
statistics/historical data.

## Features

- **Modular**: All services including the database, hub, agents, and portal can all be hosted on seperate services
- **System Stats**: Agents track system information including cpu usage, memory stats, disk usage & I/O, network
  information, temperature statistics, etc.
- **System Services**: Agents track systemctl services allowing you to stream output to the hub
- **Docker Containers**: Agents track active docker containers as well as individual container information
- **Alerts**: The hub allows you to setup alerts using various services (email, discord, telegram, etc.) for any metric
  value

## About

Lynx consists of three main services and a Postgress DB.

- lynx-agent (Individual binary to deploy on wanted servers)
- lynx-core (Central server that collects metrics and manages notifications)
- lynx-portal (Frontend app to visualize data)

The lynx-agent uses gRPC to send system info and metrics to the lynx-core. The agents also expose a websocket connection
for remote updates and command streams.

The lynx-portal is a small Svelte application that connects to the database for historical data. An if the user wants to
view live data they can connect to an agent websocket to stream commands.

## Installing the hub

It's recommended to first install and setup the hub before deploying agents\
You can do so using two methods:

**Install script**
> Use this install script to download, build, and deploy the latest version.

  ```bash
    curl -sL https://raw.githubusercontent.com/colin4683/lynx/refs/heads/master/.gitignore -o ./install-lynx-hub.sh && chmod +x ./install-lynx-hub.sh && ./install-lynx-hub.sh
  ```

**Build From Source**\
Clone and build project

  ```bash
    git clone https://github.com/colin4683/lynx.git && cd lynx/lynx-core && cargo build --release
  ```

Move binary

  ```bash
mv ./target/release/lynx-core /usr/bin/
  ```

Make service file

  ```bash
  sudo nano /etc/systemd/system/lynx-core.service
  ```

  ```bash
[Unit]
Description=lynx-core

[Service]
ExecStart=/usr/bin/lynx-core
Restart=always

[Install]
WantedBy=multi-user.target
  ```

Enable and start service

```bash
sudo systemctl enable lynx-core.service
sudo systemctl start lynx-core.service
```

## Security

### **Agent generation:**

When deploying agents the hub requests the admin user to specify the host address of the server being added. Before the
install script is generated the server is added to the database along with an agent key that is tied to the host
address. An mLTS certificate is also generated for the agent.

### **Agent verification:**

When installing an agent using the generated install script it first installs the provided certificate, then it sends a
reques to the hub asking for it to be activated. The hub verifies the provided token along with the host address that
sent the request and activates the agent. The agent then pulls the latest build and generates an deploys it using a
service file.
JWT tokens are used to verify agents which are periodically rotated.

### **Data integrity:**

Every request includes a unique SHA-256 hash which is then verified by the hub before being processed.

## ERD

![ERD](./ERD.png)
    
