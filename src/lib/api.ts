import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type PointKind = "artillery" | "target";

export interface Coord {
  x: number;
  y: number;
}

export interface Weapon {
  id: string;
  name: string;
  rangeM: [number, number];
  arcs: { id: string; name: string; table: [number, number][] }[];
}

export interface CaptureRegion {
  left: number;
  right: number;
  up: number;
  down: number;
  threshold: number;
  upscale: number;
}

export interface Settings {
  hotkeyArtillery: string;
  hotkeyTarget: string;
  weaponId: string;
  region: CaptureRegion;
}

export interface ArcSolution {
  id: string;
  name: string;
  minMil: number;
  maxMil: number;
}

export interface Solution {
  distanceM: number;
  azimuthDeg: number;
  inRange: boolean;
  arcs: ArcSolution[];
}

export interface CaptureReport {
  kind: PointKind;
  ok: boolean;
  coord: Coord | null;
  text: string;
  image: string | null;
}

export interface Snapshot {
  weapons: Weapon[];
  settings: Settings;
  artillery: Coord | null;
  target: Coord | null;
  solution: Solution | null;
  lastCapture: CaptureReport | null;
  hotkeyError: string | null;
}

export const getState = () => invoke<Snapshot>("get_state");
export const setPoint = (kind: PointKind, coord: Coord | null) => invoke("set_point", { kind, coord });
export const swapPoints = () => invoke("swap_points");
export const updateSettings = (settings: Settings) => invoke("update_settings", { settings });
export const captureNow = (kind: PointKind) => invoke("capture_now", { kind });

export const onState = (cb: (s: Snapshot) => void): Promise<UnlistenFn> =>
  listen<Snapshot>("state", (e) => cb(e.payload));
