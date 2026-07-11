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
  | "networks";
