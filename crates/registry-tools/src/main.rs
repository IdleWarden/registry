// SPDX-License-Identifier: MPL-2.0

mod asset;
mod entry;
mod index;
mod problem;
mod validate;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use asset::HttpProbe;
use validate::Schemas;

#[derive(Parser)]
#[command(name = "registry-tools", about = "IdleWarden plugin registry tooling")]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Validate {
        #[arg(long)]
        check_assets: bool,
    },
    BuildIndex {
        #[arg(long)]
        check: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool> {
    let plugins = entry::load_all(&cli.root.join("plugins"))?;
    let mods = entry::load_all(&cli.root.join("mods"))?;

    match cli.command {
        Command::Validate { check_assets } => {
            let schemas = Schemas {
                plugin: load_schema(&cli.root, "plugin-entry.schema.json")?,
                mod_entry: load_schema(&cli.root, "mod-entry.schema.json")?,
            };

            let probe = HttpProbe;
            let probe = check_assets.then_some(&probe as &dyn asset::AssetProbe);
            let problems = validate::check(&plugins, &mods, &schemas, probe);

            for problem in &problems {
                eprintln!("{}", problem.annotate());
            }
            if problems.is_empty() {
                println!(
                    "{} plugins, {} mods, no problems.",
                    plugins.len(),
                    mods.len()
                );
            }
            Ok(problems.is_empty())
        }

        Command::BuildIndex { check } => {
            let path = cli.root.join("index.json");
            let rendered = index::render(&index::build(&plugins, &mods));

            if check {
                let current = fs::read_to_string(&path)
                    .with_context(|| format!("cannot read {}", path.display()))?;
                if current == rendered {
                    println!("{} is up to date.", path.display());
                    return Ok(true);
                }
                eprintln!(
                    "{} is stale — run `registry-tools build-index` and commit the result.",
                    path.display()
                );
                return Ok(false);
            }

            fs::write(&path, &rendered)
                .with_context(|| format!("cannot write {}", path.display()))?;
            println!("{} written.", path.display());
            Ok(true)
        }
    }
}

fn load_schema(root: &Path, name: &str) -> Result<jsonschema::Validator> {
    let path = root.join("schema").join(name);
    let text =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    validate::build_validator(&serde_json::from_str(&text)?)
}
