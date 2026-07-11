<div align="center">
  <img src="assets/icon.svg" width="96" alt="DockOne" />
  <h1>DockOne</h1>
  <p><b>The Raycast for sysadmins.</b> An elegant, native desktop app to control your entire Docker infrastructure — local and remote — from a single window.</p>
</div>

---

DockOne is a cross-platform Docker control center built with **Rust + Tauri v2** and a **Svelte 5** frontend. It talks to the Docker Engine API through [`bollard`](https://crates.io/crates/bollard) over the local socket or a remote TCP endpoint — no daemon, no browser, no telemetry.

## ✨ MVP features

- **Dashboard** — overview of every host and its containers at a glance.
- **Multi‑host** — manage local and remote Docker engines side by side.
- **Container list** — full inventory with live state, image, ports and uptime.
- **Start / Stop / Restart** — one‑click control of any container.
- **Logs** — inspect the tail of a container's stdout/stderr.
- **Inspect** — complete configuration & metadata as raw JSON.
- **Image / Volume / Network managers** — browse all Docker resources.
- **Search & filter** — instant fuzzy search across hosts and containers.
- **Favorites** — pin the hosts and containers you touch the most.
- **Read‑only mode** — safe monitoring that blocks mutating actions.
- **Dark / Light theme** — system‑grade look in both.

> DockOne is designed to grow into a full infrastructure cockpit — live stats, Compose, health checks, alerts, scheduled tasks and Pro integrations (Portainer, Prometheus, Grafana, Kubernetes, Proxmox…) are on the roadmap.

## 🖥️ Connecting to hosts

| Endpoint | Example |
| --- | --- |
| Local socket (default) | `local` |
| Unix socket | `unix:///var/run/docker.sock` |
| Remote TCP | `tcp://10.0.0.5:2375` |

Remote hosts are added from **Settings → Add remote host**. Configuration is stored locally in your OS config directory (`~/.config/dockone/config.json` on Linux).

## 📦 Downloads

Prebuilt installers are published on every tagged release:

| Platform | Artifact |
| --- | --- |
| Arch Linux | `.pkg.tar.zst` |
| Linux (universal) | `.AppImage` / `.deb` |
| Windows | `.msi` and `-setup.exe` (NSIS) |
| macOS (Intel + Apple Silicon) | `.dmg` |

## 🛠️ Development

```bash
npm install
npm run tauri dev     # hot-reloading desktop app
npm run tauri build   # produce native bundles for the current OS
```

Requires the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) (Rust, Node, and the platform WebView deps).

## 🏗️ Architecture

```
src/                 Svelte 5 frontend (dashboard, containers, images, …)
src-tauri/src/
  lib.rs             Tauri commands + connection resolution
  docker.rs          bollard wrapper → serde DTOs
  state.rs           persisted hosts & settings
.github/workflows/   multi-platform release pipeline
```

## 📄 License

MIT © russus
