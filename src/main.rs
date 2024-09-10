use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use keyring::Entry;
use serde::{Deserialize, Serialize};

const PROFILE_INFO_NAME: &str = "__profile_info";

#[derive(Parser)]
struct GlobalOptions {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Set(SetOptions),
    Use(UseOptions),
}

#[derive(Args)]
struct SetOptions {
    #[arg(long)]
    profile: String,
    #[arg(long)]
    arg_name: String,
}

fn run_set(opts: &SetOptions) -> Result<()> {
    // TODO Ask for value
    let secret = "testing".to_owned();
    let mut info = get_profile_info(&opts.profile)?;
    info.ensure_member(opts.arg_name.clone());
    let entry = Entry::new(&opts.profile, &opts.arg_name)?;
    entry.set_password(&secret)?;
    upsert_profile_info(&opts.profile, &info)?;
    Ok(())
}

#[derive(Args)]
struct UseOptions {
    #[arg(long)]
    profile: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    command: Vec<String>,
}

fn run_use(opts: &UseOptions) -> Result<()> {
    let info = get_profile_info(&opts.profile)?;
    for member in info.members {
        let entry = Entry::new(&opts.profile, &member)?;
        println!("{}={}", member, entry.get_password()?);
    }
    println!("{:?}", opts.command);
    Ok(())
}

#[derive(Default, Serialize, Deserialize)]
struct ProfileInfo {
    members: Vec<String>,
}

impl ProfileInfo {
    fn ensure_member(&mut self, member: String) {
        if !self.members.contains(&member) {
            self.members.push(member);
        }
    }
}

fn get_profile_info(profile: &str) -> Result<ProfileInfo> {
    let entry = Entry::new(profile, PROFILE_INFO_NAME)?;
    let maybe_info = entry.get_secret();
    match maybe_info {
        Ok(info) => serde_json::from_slice(&info).map_err(Into::into),
        Err(keyring::Error::NoEntry) => Ok(ProfileInfo::default()),
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
        Commands::Use(useit) => run_use(&useit),
    }
}
