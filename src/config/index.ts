import Conf from "conf";
import { ConfigStore, TmuxConfig } from "@/@types";

export class ConfigManager {
  private store: Conf<ConfigStore>;

  constructor() {
    this.store = new Conf<ConfigStore>({
      projectName: "tmux-manager",
      defaults: {
        configs: {},
      },
    });
  }

  removeEntry(name: string, entryName: string): void {
    const configs = this.store.get("configs");
    if (!configs[name]) {
      throw new Error(`Config "${name}" not found`);
    }

    const config = configs[name];
    const entryIndex = config.entries.findIndex(
      (entry) => entry.entryName === entryName,
    );

    if (entryIndex === -1) {
      throw new Error(`Entry "${entryName}" not found in config "${name}"`);
    }

    config.entries.splice(entryIndex, 1);
    config.updatedAt = new Date().toISOString();
    configs[name] = config;
    this.store.set("configs", configs);
  }

  createConfig(name: string, windows: number = 2): void {
    const configs = this.store.get("configs");
    if (configs[name]) {
      throw new Error(`Config "${name}" already exists`);
    }

    configs[name] = {
      name,
      entries: [],
      windows,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    this.store.set("configs", configs);
  }

  addEntry(name: string, entryName: string, directory: string): void {
    const configs = this.store.get("configs");
    if (!configs[name]) {
      throw new Error(`Config "${name}" not found`);
    }

    const config = configs[name];
    if (config.entries.some((entry) => entry.entryName === entryName)) {
      throw new Error(
        `Entry "${entryName}" already exists in config "${name}"`,
      );
    }

    config.entries.push({ entryName, directory });
    config.updatedAt = new Date().toISOString();
    configs[name] = config;
    this.store.set("configs", configs);
  }

  getConfig(name: string): TmuxConfig {
    const config = this.store.get("configs")[name];
    if (!config) {
      throw new Error(`Config "${name}" not found`);
    }
    return config;
  }

  deleteConfig(name: string): void {
    const configs = this.store.get("configs");
    if (!configs[name]) {
      throw new Error(`Config "${name}" not found`);
    }

    delete configs[name];
    this.store.set("configs", configs);
  }

  listConfigs(): string[] {
    return Object.keys(this.store.get("configs"));
  }
}
