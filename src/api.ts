import { invoke } from "@tauri-apps/api/core";

export interface Drive {
  path: string;
  total: number;
  free: number;
}

export interface NodeDto {
  id: number;
  name: string;
  size: number;
  files: number;
  isDir: boolean;
  hasChildren: boolean;
  color: string | null;
}

export interface Children {
  items: NodeDto[];
  hiddenCount: number;
  hiddenSize: number;
}

export interface NodeInfo {
  id: number;
  name: string;
  path: string;
  size: number;
  files: number;
  isDir: boolean;
  /** Ids desde la raíz del escaneo hasta este nodo. */
  ancestors: number[];
}

export interface FileType {
  ext: number;
  name: string;
  color: string;
  size: number;
  count: number;
}

export interface ScanProgress {
  files: number;
  bytes: number;
  current: string;
}

export interface ScanDone {
  cancelled: boolean;
  root: NodeDto | null;
  path: string;
  denied: number;
  elapsedMs: number;
}

export const api = {
  initialPath: () => invoke<string | null>("initial_path"),
  listDrives: () => invoke<Drive[]>("list_drives"),
  startScan: (path: string) => invoke<void>("start_scan", { path }),
  cancelScan: () => invoke<void>("cancel_scan"),
  getNode: (id: number) => invoke<NodeDto>("get_node", { id }),
  getChildren: (id: number) => invoke<Children>("get_children", { id }),
  getFileTypes: (rootId: number) => invoke<FileType[]>("get_file_types", { rootId }),
  largestFiles: (rootId: number, ext: number | null, limit: number) =>
    invoke<NodeInfo[]>("largest_files", { rootId, ext, limit }),
  nodeInfo: (id: number) => invoke<NodeInfo | null>("node_info", { id }),
  reveal: (id: number) => invoke<void>("reveal", { id }),
  trash: (id: number) => invoke<void>("trash", { id }),
  openRecycleBin: () => invoke<void>("open_recycle_bin"),
};
