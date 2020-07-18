use std::{cmp::Ordering, hash::Hash, io::Write};

#[derive(Debug)]
pub enum ConfigItem {
    Command(String),
    Bind(String, String),
    Cvar(String, String),
}

impl ConfigItem {
    pub fn write_string(&self, mut writer: impl Write) -> std::io::Result<()> {
        match self {
            ConfigItem::Command(cmd) => writeln!(writer, "{}", cmd),
            ConfigItem::Bind(key, bind) => writeln!(writer, "bind \"{}\" \"{}\"", key, bind),
            ConfigItem::Cvar(cvar, val) => writeln!(writer, "{} \"{}\"", cvar, val),
        }
    }
}

impl Hash for ConfigItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ConfigItem::Command(cmd) => cmd.hash(state),
            ConfigItem::Bind(key, bind) => {
                key.hash(state);
                bind.hash(state);
            }
            ConfigItem::Cvar(cvar, val) => {
                cvar.hash(state);
                val.hash(state);
            }
        }
    }
}

impl PartialEq for ConfigItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ConfigItem::Command(cmd), ConfigItem::Command(other_cmd)) => cmd == other_cmd,
            (ConfigItem::Bind(key, _), ConfigItem::Bind(other_key, _)) => key == other_key,
            (ConfigItem::Cvar(cvar, _), ConfigItem::Cvar(other_cvar, _)) => cvar == other_cvar,
            _ => false,
        }
    }
}

impl Eq for ConfigItem {}

impl PartialOrd for ConfigItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (ConfigItem::Command(cmd), ConfigItem::Command(other_cmd)) => {
                cmd.partial_cmp(other_cmd)
            }
            (ConfigItem::Bind(key, _), ConfigItem::Bind(other_key, _)) => {
                key.partial_cmp(other_key)
            }
            (ConfigItem::Cvar(cvar, _), ConfigItem::Cvar(other_cvar, _)) => {
                cvar.partial_cmp(other_cvar)
            }
            (ConfigItem::Command(_), _) => Some(Ordering::Less),
            (ConfigItem::Bind(_, _), ConfigItem::Command(_)) => Some(Ordering::Greater),
            (ConfigItem::Bind(_, _), ConfigItem::Cvar(_, _)) => Some(Ordering::Less),
            (ConfigItem::Cvar(_, _), _) => Some(Ordering::Greater),
        }
    }
}

impl Ord for ConfigItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (ConfigItem::Command(cmd), ConfigItem::Command(other_cmd)) => cmd.cmp(other_cmd),
            (ConfigItem::Bind(key, _), ConfigItem::Bind(other_key, _)) => key.cmp(other_key),
            (ConfigItem::Cvar(cvar, _), ConfigItem::Cvar(other_cvar, _)) => cvar.cmp(other_cvar),
            (ConfigItem::Command(_), _) => Ordering::Less,
            (ConfigItem::Bind(_, _), ConfigItem::Command(_)) => Ordering::Greater,
            (ConfigItem::Bind(_, _), ConfigItem::Cvar(_, _)) => Ordering::Less,
            (ConfigItem::Cvar(_, _), _) => Ordering::Greater,
        }
    }
}
