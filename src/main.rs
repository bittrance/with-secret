use std::{borrow::Cow, collections::HashMap, os::unix::process::CommandExt, process::Command};

use anyhow::Result;
use clap::{builder::{styling::AnsiColor, Styles}, Args, Parser, Subcommand};
use keyring::Entry;
use rustyline::{highlight::Highlighter, Completer, Editor, Helper, Hinter, Validator};
use serde::{Deserialize, Serialize};

const PROFILE_INFO_NAME: &str = "__profile_info";
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

// TODO
// - warn when new profile is created
// - completions
// - args from stdin
// - bash exports from stdin
// - README and LICENSE

#[derive(Debug, thiserror::Error)]
enum WithError {
    #[error("Profile {0} not found")]
    ProfileNotFound(String),
    #[error("Parsing failed with '{1}' for profile: {0}")]
    InvalidProfile(String, serde_json::Error),
    #[error("Secret {0} not found in profile {1}")]
    SecretNotFound(String, String),
}

/// with-secret allows you to create profiles with key-value pairs which can then be used to run
/// commands with those pairs injected as environment varialbes. The key-value pairs are stored
/// in your local secrets service (Linux) or keyring (MacOS).
#[derive(Parser)]
#[command(styles=STYLES)]
struct GlobalOptions {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set one or more secrets in a profile
    Set(SetOptions),
    /// Clear one or more secrets from a profile
    Unset(UnsetOptions),
    /// Delete a profile, including all its secrets
    Delete(DeleteOptions),
    /// Execute a command and inject a profile's secrets into its environment
    Use(UseOptions),
}

#[derive(Args)]
struct SetOptions {
    /// Profile to work with
    #[arg(long)]
    profile: String,
    /// Name of variable to set on this profile
    #[arg(trailing_var_arg = true, required = true)]
    arg_name: Vec<String>,
}

type UnsetOptions = SetOptions; // As long as they are identical, we can cheat

#[derive(Completer, Helper, Hinter, Validator)]
struct MaskingHighlighter;

impl Highlighter for MaskingHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Owned("*".repeat(line.chars().count()))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

fn run_set(opts: &SetOptions) -> Result<()> {
    let mut rl = Editor::new()?;
    rl.set_helper(Some(MaskingHighlighter));
    let mut info = get_profile_info(&opts.profile, true)?;
    for arg_name in &opts.arg_name {
        let secret = rl.readline(&format!("Enter value for {}: ", arg_name))?;
        info.members.insert(arg_name.clone(), secret);
    }
    upsert_profile_info(&opts.profile, &info)?;
    Ok(())
}

fn run_unset(opts: &UnsetOptions) -> Result<()> {
    let mut info = get_profile_info(&opts.profile, true)?;
    for arg_name in &opts.arg_name {
        if info.members.remove(arg_name).is_none() {
            return Err(
                WithError::SecretNotFound(opts.profile.to_owned(), arg_name.to_owned()).into(),
            );
        }
    }
    upsert_profile_info(&opts.profile, &info)?;
    Ok(())
}

#[derive(Args)]
struct DeleteOptions {
    /// Profile to work with
    #[arg(long)]
    profile: String,
}

fn run_delete(opts: &DeleteOptions) -> Result<()> {
    let entry = Entry::new(&opts.profile, PROFILE_INFO_NAME)?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            Err(WithError::ProfileNotFound(opts.profile.to_owned()).into())
        }
        err => err.map_err(Into::into),
    }
}

#[derive(Args)]
struct UseOptions {
    /// Profile to work with
    #[arg(long)]
    profile: String,
    /// Command and its args to exec
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

fn run_use(opts: &UseOptions) -> Result<()> {
    let info = get_profile_info(&opts.profile, false)?;
    let mut command = Command::new(&opts.command[0]);
    command.args(&opts.command[1..]);
    for (key, secret) in info.members {
        command.env(key, secret);
    }
    Err(command.exec().into())
}

#[derive(Default, Serialize, Deserialize)]
struct ProfileInfo {
    members: HashMap<String, String>,
}

fn get_profile_info(profile: &str, autocreate: bool) -> Result<ProfileInfo> {
    let entry = Entry::new(profile, PROFILE_INFO_NAME)?;
    let maybe_info = entry.get_secret();
    match maybe_info {
        Ok(info) => serde_json::from_slice(&info).map_err(|err| {
            WithError::InvalidProfile(String::from_utf8_lossy(&info).to_string(), err).into()
        }),
        Err(keyring::Error::NoEntry) if autocreate => Ok(ProfileInfo::default()),
        Err(keyring::Error::NoEntry) => Err(WithError::ProfileNotFound(profile.to_owned()).into()),
        Err(err) => Err(err.into()),
    }
}

fn upsert_profile_info(profile: &str, info: &ProfileInfo) -> Result<()> {
    let entry = Entry::new(profile, PROFILE_INFO_NAME)?;
    let buf = serde_json::to_vec(info)?;
    entry.set_secret(&buf)?;
    Ok(())
}

fn main() -> Result<()> {
    let opts = GlobalOptions::parse();
    match opts.command {
        Commands::Set(set) => run_set(&set),
        Commands::Unset(unset) => run_unset(&unset),
        Commands::Delete(delete) => run_delete(&delete),
        Commands::Use(useit) => run_use(&useit),
    }
}
