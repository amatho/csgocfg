use std::fmt::Display;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum ConfigItem {
    Command(String),
    Bind(String, String),
    Cvar(String, String),
}

impl Display for ConfigItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigItem::Command(cmd) => write!(f, "{}", cmd),
            ConfigItem::Bind(key, bind) => write!(f, "bind \"{}\" \"{}\"", key, bind),
            ConfigItem::Cvar(cvar, val) => write!(f, "{} \"{}\"", cvar, val),
        }
    }
}
