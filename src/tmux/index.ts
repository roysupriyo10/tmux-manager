import { exec } from "child_process";
import { promisify } from "util";
import { TmuxConfig } from "@/@types";

const execAsync = promisify(exec);

export class TmuxManager {
  private tmuxPath: string = "tmux";

  async createSessions(config: TmuxConfig): Promise<void> {
    for (const entry of config.entries) {
      const sessionName = entry.entryName;

      try {
        // Check if session exists
        await execAsync(`${this.tmuxPath} has-session -t "${sessionName}"`);
        console.log(`Session ${sessionName} already exists`);
        continue;
      } catch {
        // Create new session
        await execAsync(
          `${this.tmuxPath} new-session -d -s "${sessionName}" -c "${entry.directory}"`,
        );
        console.log(`Created session: ${sessionName}`);

        // Create additional windows
        for (let i = 1; i <= config.windows; i++) {
          await execAsync(
            `${this.tmuxPath} new-window -t "${sessionName}" -c "${entry.directory}"`,
          );
          console.log(`Created window ${i} for ${sessionName}`);
        }
      }
    }
  }

  async killSessions(config: TmuxConfig): Promise<void> {
    try {
      const { stdout } = await execAsync(
        `${this.tmuxPath} list-sessions -F "#{session_name}"`,
      );
      const sessions = stdout.split("\n").filter(Boolean);

      for (const entry of config.entries) {
        if (sessions.includes(entry.entryName)) {
          await execAsync(
            `${this.tmuxPath} kill-session -t "${entry.entryName}"`,
          );
          console.log(`Killed session: ${entry.entryName}`);
        }
      }
    } catch (error) {
      console.error("No active tmux sessions found");
    }
  }
}
