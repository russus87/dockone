import { invoke } from "@tauri-apps/api/core";
import type {
  Container,
  DeploySpec,
  Df,
  DockerEvent,
  Host,
  HostSummary,
  Image,
  ImageUpdate,
  Network,
  PruneResult,
  Schedule,
  Series,
  Settings,
  Stat,
  Volume,
} from "./types";

export const api = {
  listHosts: () => invoke<Host[]>("list_hosts"),
  testHost: (name: string, endpoint: string) =>
    invoke<string>("test_host", { name, endpoint }),
  addHost: (name: string, endpoint: string) =>
    invoke<Host[]>("add_host", { name, endpoint }),
  removeHost: (id: string) => invoke<Host[]>("remove_host", { id }),
  toggleHostFavorite: (id: string) =>
    invoke<Host[]>("toggle_host_favorite", { id }),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  exportConfig: (path: string) => invoke<void>("export_config", { path }),
  importConfig: (path: string) => invoke<void>("import_config", { path }),
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

  containerStats: (hostId: string) => invoke<Stat[]>("container_stats", { hostId }),
  recentEvents: (hostId: string) => invoke<DockerEvent[]>("recent_events", { hostId }),
  systemDf: (hostId: string) => invoke<Df>("system_df", { hostId }),
  prune: (hostId: string, kind: string) =>
    invoke<PruneResult>("prune", { hostId, kind }),
  pullImage: (hostId: string, image: string) =>
    invoke<void>("pull_image", { hostId, image }),
  removeContainer: (hostId: string, id: string) =>
    invoke<void>("remove_container", { hostId, id }),
  removeImage: (hostId: string, id: string) =>
    invoke<void>("remove_image", { hostId, id }),
  removeVolume: (hostId: string, name: string) =>
    invoke<void>("remove_volume", { hostId, name }),
  removeNetwork: (hostId: string, id: string) =>
    invoke<void>("remove_network", { hostId, id }),
  execRun: (hostId: string, id: string, cmd: string) =>
    invoke<string>("exec_run", { hostId, id, cmd }),
  deployContainer: (hostId: string, spec: DeploySpec, autostart: boolean) =>
    invoke<string>("deploy_container", { hostId, spec, autostart }),
  containerConfig: (hostId: string, id: string) =>
    invoke<DeploySpec>("container_config", { hostId, id }),
  composeAction: (hostId: string, project: string, action: string) =>
    invoke<string>("compose_action", { hostId, project, action }),

  metricsHistory: (hostId: string) => invoke<Series[]>("metrics_history", { hostId }),
  checkImageUpdates: (hostId: string) =>
    invoke<ImageUpdate[]>("check_image_updates", { hostId }),

  listSchedules: () => invoke<Schedule[]>("list_schedules"),
  addSchedule: (schedule: Schedule) => invoke<Schedule[]>("add_schedule", { schedule }),
  removeSchedule: (id: string) => invoke<Schedule[]>("remove_schedule", { id }),
  toggleSchedule: (id: string) => invoke<Schedule[]>("toggle_schedule", { id }),

  allContainers: () => invoke<FlatContainer[]>("all_containers"),
  logStart: (hostId: string, id: string, tail = "300") =>
    invoke<string>("log_start", { hostId, id, tail }),
  logStop: (session: string) => invoke<void>("log_stop", { session }),
  saveText: (path: string, content: string) =>
    invoke<void>("save_text", { path, content }),
  updateContainer: (hostId: string, id: string) =>
    invoke<void>("update_container", { hostId, id }),

  fsList: (hostId: string, id: string, path: string) =>
    invoke<FsEntry[]>("fs_list", { hostId, id, path }),
  fsRead: (hostId: string, id: string, path: string) =>
    invoke<string>("fs_read", { hostId, id, path }),
  fsDownload: (hostId: string, id: string, path: string, dest: string) =>
    invoke<void>("fs_download", { hostId, id, path, dest }),
  fsUpload: (hostId: string, id: string, src: string, destDir: string) =>
    invoke<void>("fs_upload", { hostId, id, src, destDir }),
};

export interface FsEntry {
  name: string;
  kind: string; // dir | file | link
  size: number;
}

export interface FlatContainer {
  host_id: string;
  host_name: string;
  id: string;
  name: string;
  image: string;
  state: string;
}

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
