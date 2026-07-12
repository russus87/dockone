<div align="center">
  <img src="assets/icon.svg" width="96" alt="DockOne" />
  <h1>DockOne</h1>
  <p><b>The Raycast for sysadmins.</b> An elegant, native desktop app to control your entire Docker infrastructure — local and remote — from a single window.</p>

  <p>
    <a href="https://github.com/russus87/dockone/actions/workflows/build.yml"><img src="https://github.com/russus87/dockone/actions/workflows/build.yml/badge.svg" alt="Build" /></a>
    <a href="https://github.com/russus87/dockone/releases/latest"><img src="https://img.shields.io/github/v/release/russus87/dockone?color=2f6cf0&label=release" alt="Release" /></a>
    <a href="https://github.com/russus87/dockone/releases"><img src="https://img.shields.io/github/downloads/russus87/dockone/total?color=17a95c&label=downloads" alt="Downloads" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/russus87/dockone?color=gray" alt="License" /></a>
    <img src="https://img.shields.io/badge/platforms-Linux%20%C2%B7%20Windows%20%C2%B7%20macOS-6aa0ff" alt="Platforms" />
    <img src="https://img.shields.io/badge/built%20with-Tauri%20v2%20%C2%B7%20Rust%20%C2%B7%20Svelte%205-ffb454" alt="Stack" />
  </p>
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
- **Live Stats** — real‑time CPU %, memory, network and disk I/O per container, auto‑refreshing.
- **Events timeline** — the last hours of Docker events (create/start/die/pull…) colour‑coded.
- **Image manager** — list, **pull** from any registry, and remove images.
- **Volume / Network managers** — browse and remove Docker resources.
- **Maintenance** — disk‑usage breakdown + one‑click **prune** (containers, images, volumes, networks).
- **Deploy container** — guided creation: image, name, ports, volumes, env, restart policy → create & start.
- **Clone container** — duplicate any container's configuration into the deploy form in one click.
- **Bulk actions** — start / stop / restart / remove many containers at once.
- **Container control** — start, stop, restart, **pause, unpause, kill**.
- **Exec** — run a command inside a running container and see its output.
- **Alerts** — background watcher that fires a desktop notification when a container **crashes or turns unhealthy**.
- **Health & stacks** — health status badges and docker‑compose project grouping.
- **Test connection** — probe a host (local or remote) and see its Docker version before saving it.
- **Export / Import** — back up and restore your hosts & settings to a JSON file.
- **Search & filter** — instant fuzzy search across hosts and containers.
- **Favorites** — pin the hosts and containers you touch the most.
- **Read‑only mode** — safe monitoring that blocks mutating actions.
- **Dark / Light theme** — system‑grade look in both.

> DockOne is designed to grow into a full infrastructure cockpit — an interactive streaming terminal, Compose up/down, scheduled tasks, metrics history and Pro integrations (Portainer, Prometheus, Grafana, Kubernetes, Proxmox…) are on the roadmap.

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
