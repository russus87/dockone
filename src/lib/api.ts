import { invoke } from "@tauri-apps/api/core";
import type {
  Container,
  Host,
  HostSummary,
  Image,
  Network,
  Settings,
  Volume,
} from "./types";

export const api = {
  listHosts: () => invoke<Host[]>("list_hosts"),
  addHost: (name: string, endpoint: string) =>
    invoke<Host[]>("add_host", { name, endpoint }),
  removeHost: (id: string) => invoke<Host[]>("remove_host", { id }),
  toggleHostFavorite: (id: string) =>
    invoke<Host[]>("toggle_host_favorite", { id }),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  toggleFavoriteContainer: (key: string) =>
    invoke<Settings>("toggle_favorite_container", { key }),

  dashboard: () => invoke<HostSummary[]>("dashboard"),
  containers: (hostId: string) =>
    invoke<Container[]>("get_containers", { hostId }),
  images: (hostId: string) => invoke<Image[]>("get_images", { hostId }),
  volumes: (hostId: string) => invoke<Volume[]>("get_volumes", { hostId }),
  networks: (hostId: string) => invoke<Network[]>("get_networks", { hostId }),

  containerAction: (hostId: string, id: string, action: string) =>
    invoke<void>("container_action", { hostId, id, action }),
  containerLogs: (hostId: string, id: string, tail = "200") =>
    invoke<string>("container_logs", { hostId, id, tail }),
  inspectContainer: (hostId: string, id: string) =>
    invoke<unknown>("inspect_container", { hostId, id }),
};

export function humanBytes(n: number): string {
  if (!n || n < 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 && i > 0 ? 1 : 0)} ${units[i]}`;
}

export function ago(unixSeconds: number): string {
  if (!unixSeconds) return "—";
  const diff = Date.now() / 1000 - unixSeconds;
  const d = Math.floor(diff / 86400);
  if (d > 0) return `${d}d fa`;
  const h = Math.floor(diff / 3600);
  if (h > 0) return `${h}h fa`;
  const m = Math.floor(diff / 60);
  if (m > 0) return `${m}m fa`;
  return "adesso";
}
