mod config;
mod parser;

use anyhow::{anyhow, Context, Result};
use config::ConfigItem;
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

enum Command {
    Patch { target: PathBuf, patch: PathBuf },
    Validate { target: PathBuf },
    Unrecognized(String),
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command> {
    let mut args = args.into_iter();

    let command = args.next().ok_or_else(|| anyhow!("no command specified"))?;

    let command = match &command[..] {
        "patch" => {
            let target_path = args.next().ok_or_else(|| anyhow!("no target specified"))?;
            let target = Path::new(&target_path)
                .canonicalize()
                .with_context(|| format!("could not find the file {:?}", target_path))?;

            let patch_path = args.next().ok_or_else(|| anyhow!("no patch specified"))?;
            let patch = Path::new(&patch_path)
                .canonicalize()
                .with_context(|| format!("could not find the file {:?}", patch_path))?;

            Command::Patch { target, patch }
        }
        "validate" => {
            let target_path = args.next().ok_or_else(|| anyhow!("no target specified"))?;
            let target = Path::new(&target_path)
                .canonicalize()
                .with_context(|| format!("could not find the file {:?}", target_path))?;

            Command::Validate { target }
        }
        _ => Command::Unrecognized(command),
    };

    Ok(command)
}

fn apply_patch(target: PathBuf, patch: PathBuf) -> Result<()> {
    let mut config_set: BTreeSet<ConfigItem> = BTreeSet::new();

    let target_reader = BufReader::new(File::open(&target)?);
    for line in target_reader.lines() {
        let line = line?;
        let config_item = parser::parse_line(&line)?;

        if let Some(item) = config_item {
            config_set.insert(item);
        }
    }

    let patch_reader = BufReader::new(File::open(&patch)?);
    for line in patch_reader.lines() {
        let line = line?;
        let config_item = parser::parse_line(&line)?;

        if let Some(item) = config_item {
            config_set.replace(item);
        }
    }

    let mut target_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&target)?;
    for value in config_set {
        value.write_string(&mut target_file)?;
    }

    Ok(())
}

fn validate(target: PathBuf) -> Result<()> {
    let target_reader = BufReader::new(File::open(&target)?);
    for line in target_reader.lines() {
        let line = line?;
        parser::parse_line(&line)?;
    }

    println!("Config `{}` is valid.", target.display());

    Ok(())
}

pub fn run() -> Result<()> {
    let command = parse_args(std::env::args().skip(1))?;

    match command {
        Command::Patch { target, patch } => apply_patch(target, patch)?,
        Command::Validate { target } => validate(target)?,
        Command::Unrecognized(s) => return Err(anyhow!("unrecognized command, {:?}", s)),
    }

    Ok(())
}
