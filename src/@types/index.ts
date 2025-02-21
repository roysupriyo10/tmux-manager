export interface TmuxEntry {
  entryName: string;
  directory: string;
}

export interface TmuxConfig {
  name: string;
  entries: TmuxEntry[];
  windows: number;
  createdAt: string;
  updatedAt: string;
}

export interface ConfigStore {
    [key: string]: TmuxConfig;
}
