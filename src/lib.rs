mod config;
mod parser;

use config::ConfigItem;
use parser::ParseError;
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unrecognized command `{0}`")]
    UnrecognizedCommand(String),
    #[error("missing argument `{0}`")]
    MissingArgument(&'static str),
    #[error("file not found `{0}`")]
    FileNotFound(String),
    #[error("error reading file, {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("parse error at line {line_number},\n{source}")]
    ParseError {
        source: ParseError,
        line_number: usize,
    },
}

/// Turns a tuple of a `ParseError` and a zero-based index into `Error::ParseError`
impl From<(ParseError, usize)> for Error {
    fn from((error, line): (ParseError, usize)) -> Self {
        Error::ParseError {
            source: error,
            line_number: line + 1,
        }
    }
}

enum Command {
    Patch { target: PathBuf, patch: PathBuf },
    Validate { target: PathBuf },
    Unrecognized(String),
}

pub fn run() -> Result<(), Error> {
    let command = parse_args(std::env::args().skip(1))?;

    match command {
        Command::Patch { target, patch } => apply_patch(target, patch)?,
        Command::Validate { target } => validate(target)?,
        Command::Unrecognized(s) => return Err(Error::UnrecognizedCommand(s)),
    }

    Ok(())
}

fn apply_patch(target: PathBuf, patch: PathBuf) -> Result<(), Error> {
    let mut config_set: BTreeSet<ConfigItem> = BTreeSet::new();

    let target_reader = BufReader::new(File::open(&target)?);
    for (index, line) in target_reader.lines().enumerate() {
        let line = line?;
        let config_item = parser::parse_line(&line).map_err(|e| (e, index))?;

        if let Some(item) = config_item {
            config_set.insert(item);
        }
    }

    let patch_reader = BufReader::new(File::open(&patch)?);
    for (index, line) in patch_reader.lines().enumerate() {
        let line = line?;
        let config_item = parser::parse_line(&line).map_err(|e| (e, index))?;

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

    println!(
        "Successfully patched `{}` onto `{}`.",
        patch.display(),
        target.display()
    );

    Ok(())
}

fn validate(target: PathBuf) -> Result<(), Error> {
    let target_reader = BufReader::new(File::open(&target)?);
    for (index, line) in target_reader.lines().enumerate() {
        let line = line?;
        parser::parse_line(&line).map_err(|e| (e, index))?;
    }

    println!("Config `{}` is valid.", target.display());

    Ok(())
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, Error> {
    let mut args = args.into_iter();

    let command = args
        .next()
        .ok_or_else(|| Error::UnrecognizedCommand("no command".to_owned()))?;

    let command = match &command[..] {
        "patch" => {
            let target_path = args
                .next()
                .ok_or_else(|| Error::MissingArgument("target"))?;
            let target = Path::new(&target_path)
                .canonicalize()
                .map_err(|_| Error::FileNotFound(target_path))?;

            let patch_path = args.next().ok_or_else(|| Error::MissingArgument("patch"))?;
            let patch = Path::new(&patch_path)
                .canonicalize()
                .map_err(|_| Error::FileNotFound(patch_path))?;

            Command::Patch { target, patch }
        }
        "validate" => {
            let target_path = args
                .next()
                .ok_or_else(|| Error::MissingArgument("target"))?;
            let target = Path::new(&target_path)
                .canonicalize()
                .map_err(|_| Error::FileNotFound(target_path))?;

            Command::Validate { target }
        }
        _ => Command::Unrecognized(command),
    };

    Ok(command)
}
