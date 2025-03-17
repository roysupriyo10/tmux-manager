#!/usr/bin/env node
import { Command } from "commander";
import { ConfigManager } from "./config";
import { TmuxManager } from "./tmux";
import chalk from "chalk";
import { findBestMatch } from "./utils";
import { TmuxConfig } from "./@types";

const program = new Command();
const configManager = new ConfigManager();
const tmuxManager = new TmuxManager();

program
  .name("tmux-manager")
  .description("CLI tool for managing tmux sessions")
  .version("1.0.0");

program
  .command("create")
  .description("Create a new tmux configuration")
  .requiredOption("--name <name>", "name of the configuration")
  .option("--windows <number>", "number of windows per session", "2")
  .action(async (options) => {
    try {
      configManager.createConfig(options.name, parseInt(options.windows));
      console.log(chalk.green(`Created configuration: ${options.name}`));
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      console.error(chalk.red(errorMessage));
      process.exit(1);
    }
  });

program
  .command("entry")
  .description("Add an entry to a configuration")
  .requiredOption("--name <name>", "name of the configuration")
  .requiredOption("--entry-name <entry-name>", "name of the entry")
  .requiredOption("--entry-dir <entry-dir>", "directory for the entry")
  .action(async (options) => {
    try {
      configManager.addEntry(options.name, options.entryName, options.entryDir);
      console.log(
        chalk.green(`Added entry ${options.entryName} to ${options.name}`),
      );
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      console.error(chalk.red(errorMessage));
      process.exit(1);
    }
  });

// Updated start command section
program
  .command("start")
  .description("Start tmux sessions from a configuration")
  .requiredOption("--name <name>", "name of the configuration")
  .option("--entry <entries...>", "specific entries to start (fuzzy matching)")
  .action(async (options) => {
    try {
      const config = configManager.getConfig(options.name);

      if (
        options.entry &&
        Array.isArray(options.entry) &&
        options.entry.length > 0
      ) {
        // Fuzzy match the entry names
        const fuzzyMatchedConfig = {
          ...config,
          entries: [] as TmuxConfig["entries"],
        };

        for (const entryPattern of options.entry) {
          const allEntryNames = config.entries.map((entry) => entry.entryName);
          const matchedEntryName = findBestMatch(entryPattern, allEntryNames);

          if (matchedEntryName) {
            const matchedEntry = config.entries.find(
              (entry) => entry.entryName === matchedEntryName,
            );
            if (matchedEntry) {
              fuzzyMatchedConfig.entries.push(matchedEntry);
              console.log(
                chalk.green(
                  `Fuzzy matched '${entryPattern}' to '${matchedEntryName}'`,
                ),
              );
            }
          } else {
            console.log(chalk.yellow(`No match found for '${entryPattern}'`));
          }
        }

        if (fuzzyMatchedConfig.entries.length === 0) {
          console.log(
            chalk.yellow("No entries matched. No sessions will be created."),
          );
          return;
        }

        await tmuxManager.createSessions(fuzzyMatchedConfig);
      } else {
        await tmuxManager.createSessions(config);
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      console.error(chalk.red(errorMessage));
      process.exit(1);
    }
  });

// program
//   .command("start")
//   .description("Start tmux sessions from a configuration")
//   .requiredOption("--name <name>", "name of the configuration")
//   .action(async (options) => {
//     try {
//       const config = configManager.getConfig(options.name);
//       await tmuxManager.createSessions(config);
//     } catch (error) {
//       const errorMessage =
//         error instanceof Error ? error.message : String(error);
//       console.error(chalk.red(errorMessage));
//       process.exit(1);
//     }
//   });

program
  .command("entry-remove")
  .description("Remove an entry from a configuration")
  .requiredOption("--name <name>", "name of the configuration")
  .requiredOption("--entry-name <entry-name>", "name of the entry to remove")
  .action(async (options) => {
    try {
      configManager.removeEntry(options.name, options.entryName);
      console.log(chalk.green);
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      console.error(chalk.red(errorMessage));
      process.exit(1);
    }
  });

program
  .command("kill")
  .description("Kill tmux sessions from a configuration")
  .requiredOption("--name <name>", "name of the configuration")
  .option("--entry <entries...>", "specific entries to kill (fuzzy matching)")
  .action(async (options) => {
    try {
      const config = configManager.getConfig(options.name);

      if (options.entry && Array.isArray(options.entry) && options.entry.length > 0) {
        const fuzzyMatchedConfig = {
          ...config,
          entries: [] as TmuxConfig["entries"],
        };

        for (const entryPattern of options.entry) {
          const allEntryNames = config.entries.map((entry) => entry.entryName);
          const matchedEntryName = findBestMatch(entryPattern, allEntryNames);

          if (matchedEntryName) {
            const matchedEntry = config.entries.find(
              (entry) => entry.entryName === matchedEntryName,
            );
            if (matchedEntry) {
              fuzzyMatchedConfig.entries.push(matchedEntry);
              console.log(
                chalk.green(
                  `Fuzzy matched '${entryPattern}' to '${matchedEntryName}'`,
                ),
              );
            }
          }
          else {
            console.log(chalk.yellow(`No match found for '${entryPattern}'`));
          }
        }

        if (fuzzyMatchedConfig.entries.length === 0) {
          console.log(chalk.yellow("No entries matched. No sessions will be killed."));
          return;
        }

        await tmuxManager.killSessions(fuzzyMatchedConfig);
      }
      else {
        await tmuxManager.killSessions(config);
      }

    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      console.error(chalk.red(errorMessage));
      process.exit(1);
    }
  });

program
  .command("delete")
  .description("Delete a configuration")
  .requiredOption("--name <name>", "name of the configuration")
  .action((options) => {
    try {
      configManager.deleteConfig(options.name);
      console.log(chalk.green(`Deleted configuration: ${options.name}`));
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      console.error(chalk.red(errorMessage));
      process.exit(1);
    }
  });

program
  .command("list")
  .description("List all configurations")
  .action(() => {
    const configs = configManager.listConfigs();
    if (configs.length === 0) {
      console.log(chalk.yellow("No configurations found"));
      return;
    }
    console.log(chalk.blue("Available configurations:"));
    configs.forEach((name) => {
      const config = configManager.getConfig(name);
      console.log(chalk.green(`\n${name}:`));
      console.log(`  Windows: ${config.windows}`);
      console.log("  Entries:");
      config.entries.forEach((entry) => {
        console.log(`    - ${entry.entryName}: ${entry.directory}`);
      });
    });
  });

program.parse();
