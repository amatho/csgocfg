use anyhow::{anyhow, bail, Context, Result};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
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

impl<'a> ConfigItem<'a> {
    fn push_to_string(&self, string: &mut String) {
        match self {
            ConfigItem::Cmd(cmd) => {
                string.push_str(cmd);
            }
            ConfigItem::CmdWithArgs(cmd, arg1, arg2) => {
                string.push_str(cmd);
                string.push(' ');
                string.push_str(arg1);
                string.push(' ');
                string.push_str(arg2);
            }
            ConfigItem::Cvar(cvar, val) => {
                string.push_str(cvar);
                string.push(' ');
                string.push_str(val);
            }
            ConfigItem::Empty => (),
        }

        string.push('\n');
    }
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
    let line = line.trim();

    if line.is_empty() {
        return Ok(ConfigItem::Empty);
    }

    let mut parts = Vec::new();
    let mut parsing_quotes = false;
    let mut start = 0;
    for (index, c) in line.char_indices() {
        if !parsing_quotes && c == '"' {
            parsing_quotes = true;
        } else if !parsing_quotes && c == ' ' {
            parts.push(&line[start..index]);
            start = index + 1;
        } else if !parsing_quotes && c == '/' {
            if &line[index + 1..index + 2] == "/" {
                break;
            }
        } else if parsing_quotes && c == '"' {
            parsing_quotes = false;
        }
    }

    parts.push(&line[start..]);

    let item = match parts[..] {
        [cmd] => ConfigItem::Cmd(cmd),
        [cvar, val] => ConfigItem::Cvar(cvar, val),
        [cmd, arg1, arg2] => ConfigItem::CmdWithArgs(cmd, arg1, arg2),
        _ => {
            dbg!(parts);
            bail!("invalid config line: {}", line)
        }
    };

    Ok(item)
}

fn apply_patch(target: PathBuf, patch: PathBuf) -> Result<()> {
    let mut patch_file = File::open(patch)?;
    let mut patch_contents = String::new();
    patch_file.read_to_string(&mut patch_contents)?;
    drop(patch_file);

    let mut patch_lookup: HashMap<&str, ConfigItem> = HashMap::new();

    for line in patch_contents.lines() {
        let config_item = parse_config_line(&line)?;

        match config_item {
            ConfigItem::Cmd(cmd) => {
                patch_lookup.insert(cmd, config_item);
            }
            ConfigItem::CmdWithArgs(cmd, ..) => {
                patch_lookup.insert(cmd, config_item);
            }
            ConfigItem::Cvar(cvar, ..) => {
                patch_lookup.insert(cvar, config_item);
            }
            ConfigItem::Empty => (),
        }
    }

    let target_reader = BufReader::new(File::open(&target)?);
    let mut new_contents = String::new();

    for line in target_reader.lines() {
        let line = line?;
        let config_item = parse_config_line(&line)?;

        let new_config_item = match config_item {
            ConfigItem::Cmd(cmd) => patch_lookup.remove(cmd).unwrap_or(config_item),
            ConfigItem::CmdWithArgs(cmd, ..) => patch_lookup.remove(cmd).unwrap_or(config_item),
            ConfigItem::Cvar(cvar, ..) => patch_lookup.remove(cvar).unwrap_or(config_item),
            ConfigItem::Empty => config_item,
        };

        new_config_item.push_to_string(&mut new_contents);
    }

    for (_, value) in patch_lookup {
        value.push_to_string(&mut new_contents);
    }

    let mut target_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&target)?;
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
