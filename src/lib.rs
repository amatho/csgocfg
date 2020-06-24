use anyhow::{anyhow, ensure, Context, Result};
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

fn parse_config_line(line: &str) -> Result<(String, String)> {
    let mut split_index = None;
    let mut chars = line.chars().peekable();
    let mut index = 0;

    while let Some(c) = chars.next() {
        if c == '/' && chars.peek().map_or(false, |next| *next == '/') {
            break;
        } else if split_index.is_none() && c == ' ' {
            split_index = Some(index);
        }

        index += 1;
    }

    ensure!(split_index.is_some(), "invalid line in config");

    let split = split_index.unwrap();
    let cvar = line[..split].to_owned();
    let val = line[split + 1..index].to_owned();

    Ok((cvar, val))
}

fn apply_patch(target: PathBuf, patch: PathBuf) -> Result<()> {
    let patch_file = BufReader::new(File::open(patch)?);
    let mut patch_lookup: HashMap<String, String> = HashMap::new();

    for line in patch_file.lines() {
        let line = line?;
        let (cvar, val) = parse_config_line(&line)?;
        patch_lookup.insert(cvar, val);
    }

    let target_reader = BufReader::new(File::open(&target)?);
    let mut new_contents = String::new();

    for line in target_reader.lines() {
        let line = line?;
        let split_index = line.find(' ').context("invalid line in config")?;
        let cvar = &line[..split_index];
        let mut val = &line[split_index + 1..];
        if let Some(index) = val.find("//") {
            val = &val[..index];
        }

        if patch_lookup.contains_key(cvar) {
            val = patch_lookup.get(cvar).map_or(val, |v| v);
        }

        new_contents.push_str(cvar);
        new_contents.push_str(" ");
        new_contents.push_str(val);
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
