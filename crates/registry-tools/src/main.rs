// SPDX-License-Identifier: MPL-2.0

mod asset;
mod entry;
mod index;
mod problem;
mod validate;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use asset::HttpProbe;

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
    let entries = entry::load_all(&cli.root.join("plugins"))?;

    match cli.command {
        Command::Validate { check_assets } => {
            let schema_path = cli.root.join("schema/plugin-entry.schema.json");
            let schema = fs::read_to_string(&schema_path)
                .with_context(|| format!("cannot read {}", schema_path.display()))?;
            let validator = validate::build_validator(&serde_json::from_str(&schema)?)?;

            let probe = HttpProbe;
            let probe = check_assets.then_some(&probe as &dyn asset::AssetProbe);
            let problems = validate::check(&entries, &validator, probe);

            for problem in &problems {
                eprintln!("{}", problem.annotate());
            }
            if problems.is_empty() {
                println!("{} entries, no problems.", entries.len());
            }
            Ok(problems.is_empty())
        }

        Command::BuildIndex { check } => {
            let path = cli.root.join("index.json");
            let rendered = index::render(&index::build(&entries));

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
            println!("{} written with {} entries.", path.display(), entries.len());
            Ok(true)
        }
    }
}
