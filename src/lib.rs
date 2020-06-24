use anyhow::{anyhow, bail, ensure, Context, Result};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

enum Command {
    Patch { target: PathBuf, patch: PathBuf },
    Unrecognized(String),
}

enum ConfigItem<'a> {
    Cmd(&'a str),
    CmdWithArgs(&'a str, &'a str, &'a str),
    Cvar(&'a str, &'a str),
    Empty,
}

fn parse_args<T>(args: T) -> Result<Command>
where
    T: IntoIterator,
    T::Item: AsRef<str>,
{
    let mut args = args.into_iter();

    let command = match args
        .next()
        .ok_or_else(|| anyhow!("no command specified"))?
        .as_ref()
    {
        "patch" => {
            let target_path = args.next().ok_or_else(|| anyhow!("no target specified"))?;
            let target = Path::new(target_path.as_ref())
                .canonicalize()
                .with_context(|| format!("could not find the file {:?}", target_path.as_ref()))?;

            let patch_path = args.next().ok_or_else(|| anyhow!("no patch specified"))?;
            let patch = Path::new(patch_path.as_ref())
                .canonicalize()
                .with_context(|| format!("could not find the file {:?}", patch_path.as_ref()))?;

            Command::Patch { target, patch }
        }
        s => Command::Unrecognized(s.to_owned()),
    };

    Ok(command)
}

fn parse_config_line(line: &str) -> Result<ConfigItem> {
    let line = if let Some(index) = line.find("//") {
        &line[..index]
    } else {
        &line
    };

    if line.is_empty() {
        return Ok(ConfigItem::Empty);
    }

    let parts: Vec<_> = line.split_whitespace().collect();

    let item = match parts.len() {
        1 => ConfigItem::Cmd(parts[0]),
        2 => {
            let cvar = parts[0];
            let val = parts[1];

            ConfigItem::Cvar(cvar, val)
        }
        3 => {
            let cmd = parts[0];
            let arg1 = parts[1];
            let arg2 = parts[2];

            ConfigItem::CmdWithArgs(cmd, arg1, arg2)
        }
    };

    Ok(item)
}

fn apply_patch(target: PathBuf, patch: PathBuf) -> Result<()> {
    let patch_file = BufReader::new(File::open(patch)?);
    let mut patch_lookup: HashMap<String, ConfigItem> = HashMap::new();

    for line in patch_file.lines() {
        let line = line?;
        let config_item = parse_config_line(&line)?;

        match config_item {
            ConfigItem::Cmd(cmd) => {
                patch_lookup.insert(cmd.to_owned(), config_item);
            }
            ConfigItem::CmdWithArgs(cmd, ..) => {
                patch_lookup.insert(cmd.to_owned(), config_item);
            }
            ConfigItem::Cvar(cvar, ..) => {
                patch_lookup.insert(cvar.to_owned(), config_item);
            }
            ConfigItem::Empty => (),
        }
    }

    let target_reader = BufReader::new(File::open(&target)?);
    let mut new_contents = String::new();

    for line in target_reader.lines() {
        let line = line?;
        let config_item = parse_config_line(&line)?;

        match config_item {}
    }

    let mut target_file = File::create(&target)?;
    target_file.write_all(new_contents.as_bytes())?;

    Ok(())
}

pub fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let command = parse_args(&args[1..])?;

    match command {
        Command::Patch { target, patch } => apply_patch(target, patch)?,
        Command::Unrecognized(s) => return Err(anyhow!("unrecognized command, {:?}", s)),
    }

    Ok(())
}
