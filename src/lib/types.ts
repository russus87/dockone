export interface Host {
  id: string;
  name: string;
  endpoint: string;
  favorite: boolean;
}

export interface Settings {
  theme: "dark" | "light";
  read_only: boolean;
  favorite_containers: string[];
  alerts_enabled: boolean;
  alert_poll_secs: number;
  metrics_enabled: boolean;
}

export interface Sample {
  t: number;
  cpu: number;
  mem: number;
  mem_limit: number;
}

export interface Series {
  name: string;
  points: Sample[];
}

export interface Schedule {
  id: string;
  host_id: string;
  host_name: string;
  container: string;
  action: string;
  kind: string; // daily | interval
  time: string; // HH:MM
  every_min: number;
  enabled: boolean;
  last_run: number;
}

export interface ImageUpdate {
  tag: string;
  up_to_date: boolean | null;
  error: string | null;
}

export interface PortMap {
  host: string;
  container: string;
  proto: string;
}

export interface VolMap {
  host: string;
  container: string;
}

export interface DeploySpec {
  name: string;
  image: string;
  cmd: string[];
  env: string[];
  ports: PortMap[];
  volumes: VolMap[];
  restart: string;
}

export interface HostSummary {
  id: string;
  name: string;
  online: boolean;
  error: string | null;
  containers: number;
  running: number;
  stopped: number;
  paused: number;
  images: number;
  cpus: number;
  mem_total: number;
  docker_version: string | null;
  os: string | null;
}

export interface Container {
  id: string;
  name: string;
  image: string;
  state: string;
  status: string;
  ports: string[];
  created: number;
  health: string;
  compose: string | null;
}

export interface Stat {
  id: string;
  name: string;
  cpu_percent: number;
  mem_used: number;
  mem_limit: number;
  mem_percent: number;
  net_rx: number;
  net_tx: number;
  blk_read: number;
  blk_write: number;
}

export interface DockerEvent {
  time: number;
  kind: string;
  action: string;
  actor: string;
}

export interface Df {
  images_size: number;
  images_count: number;
  volumes_size: number;
  volumes_count: number;
  containers_count: number;
}

export interface PruneResult {
  reclaimed: number;
  detail: string;
}

export interface Image {
  id: string;
  tags: string[];
  size: number;
  created: number;
  containers: number;
}

export interface Volume {
  name: string;
  driver: string;
  mountpoint: string;
  created: string | null;
}

export interface Network {
  id: string;
  name: string;
  driver: string;
  scope: string;
}

export type View =
  | "dashboard"
  | "containers"
  | "images"
  | "volumes"
  | "networks"
  | "stats"
  | "metrics"
  | "events"
  | "maintenance"
  | "schedules";
