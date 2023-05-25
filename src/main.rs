mod diff;
mod nix_parse;
mod nix_read;
mod nix_write;

use std::{fmt, fs, io, env};
use std::env::VarError;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{PathBuf};
use std::process::{Command, ExitCode};
use owo_colors::{OwoColorize};
use clap::{Parser, Subcommand};
use indexmap::IndexSet;
use thiserror::Error;

const QUERY: &str = "home.packages";

#[derive(Subcommand)]
enum HdnSubcommand {
    /// Add packages to home.nix, then run home-manager switch
    Add {
        /// The packages to add, space separated
        packages: Vec<String>,
        /// Passes --show-trace to home-manager switch
        #[clap(long, short, action)]
        show_trace: bool
    },
    /// Remove packages from home.nix, then run home-manager switch
    Remove {
        /// The packages to remove, space separated
        packages: Vec<String>,
        /// Passes --show-trace to home-manager switch
        #[clap(long, short, action)]
        show_trace: bool
    }
}

#[derive(Parser)]
#[command(author = "Fisher Sun")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct HdnCli {
    #[command(subcommand)]
    subcommand: HdnSubcommand,
}

#[derive(Error, Debug)]
enum RunHomeManagerSwitchError {
    #[error("Could not run home-manager switch")]
    CouldNotRun(#[source] io::Error),
    #[error("OS error occurred while running home-manager switch")]
    OSError(#[source] io::Error),
    #[error("home-manager switch returned a non-zero exit code")]
    Unsuccessful
}

fn run_home_manager_switch(show_trace: &bool) -> Result<(), RunHomeManagerSwitchError> {
    use crate::RunHomeManagerSwitchError::*;

    let mut command = Command::new("home-manager");
    let command = command.arg("switch");
    let command = if *show_trace {command.arg("--show-trace")} else {command};

    let mut child = command
        .spawn()
        .map_err(CouldNotRun)?;

    let exit_status = child.wait().map_err(OSError)?;

    if !exit_status.success() {
        return Err(Unsuccessful);
    }
    Ok(())
}

enum UpdateNixMode {
    Add,
    Remove
}

#[derive(Error, Debug)]
enum UpdateNixError {
    #[error("could not read values of home.packages attribute in home.nix")]
    CouldNotReadNix(#[source] nix_read::ReadError),
    #[error("could not write home.packages attribute for new packages")]
    CouldNotWriteNix(#[source] nix_write::WriteError),
}

fn update_nix(content: &str, packages: &Vec<String>, mode: &UpdateNixMode) -> Result<String, UpdateNixError> {
    use crate::UpdateNixError::*;
    use crate::UpdateNixMode::*;

    let packages: IndexSet<&String> = IndexSet::from_iter(packages);

    let existing_packages: IndexSet<String> = IndexSet::from_iter(
        nix_read::getarrvals(content, QUERY)
            .map_err(CouldNotReadNix)?
    );

    match mode {
        Add => {
            let transformed_packages: Vec<&String> = packages.into_iter()
                .filter(|&p| !existing_packages.contains(p))
                .collect();

            nix_write::addtoarr(
                content,
                QUERY,
                transformed_packages.into_iter().cloned().collect()
            ).map_err(CouldNotWriteNix)
        }
        Remove => {
            let transformed_packages: Vec<&String> = packages.into_iter()
                .filter(|&p| existing_packages.contains(p))
                .collect();

            nix_write::rmarr(
                content,
                QUERY,
                transformed_packages.into_iter().cloned().collect()
            ).map_err(CouldNotWriteNix)
        }
    }
}

#[derive(Error, Debug)]
enum GetHomeDotNixError {
    #[error("could not get $HOME environment variable")]
    NoHomeEnvironmentVariable(#[source] VarError),
    #[error("home.nix was not found in any of the default locations")]
    NotFound
}

fn get_home_dot_nix() -> Result<PathBuf, GetHomeDotNixError> {
    use crate::GetHomeDotNixError::*;

    let config_home = env::var("XDG_CONFIG_HOME");
    let config_home: PathBuf = match config_home {
        Ok(s) => PathBuf::from(s),
        Err(_error) =>
            [env::var("HOME").map_err(NoHomeEnvironmentVariable)?, ".config".to_string()]
                .iter()
                .collect()
    };

    let paths_to_check = [
        config_home.join("home-manager/home.nix"),
        config_home.join("nixpkgs/home.nix"),
        [env::var("HOME").map_err(NoHomeEnvironmentVariable)?, ".nixpkgs/home.nix".to_string()]
            .iter()
            .collect(),
    ];

    for path in paths_to_check {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(NotFound)
}

enum HdnSuccess {
    HomeManagerSwitchSucceeded,
    HomeManagerSwitchErroredButRollbackSuccessful,
    NothingToAdd,
    NothingToRemove
}

impl Display for HdnSuccess {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use crate::HdnSuccess::*;
        match self {
            HomeManagerSwitchSucceeded => {
                write!(f, "Successfully updated home.nix and activated generation")
            },
            HomeManagerSwitchErroredButRollbackSuccessful => {
                write!(f, "Running home-manager switch errored; your home.nix has been rolled back")
            }
            NothingToAdd => {
                write!(f, "home.nix already contains all the specified packages; home-manager switch was not run")
            }
            NothingToRemove => {
                write!(f, "home.nix doesn't contain any of the specified packages, home-manager switch was not run")
            }
        }
    }
}

#[derive(Error, Debug)]
enum HdnError {
    #[error("could not find home.nix")]
    CouldNotFindHomeDotNix(#[source] GetHomeDotNixError),
    #[error("could not read home.nix")]
    CouldNotReadFile(#[source] io::Error),
    #[error("could not write to home.nix")]
    CouldNotWriteToFile(#[source] io::Error),
    #[error("running home-manager switch errored, and during the rollback of home.nix, another error occurred")]
    UnsuccessfulAndNotRolledBack(#[source] io::Error),
    #[error("could not update home.packages attribute in home.nix")]
    CouldNotUpdatePackages(#[source] UpdateNixError),
}

fn update(mode: UpdateNixMode, packages: &Vec<String>, show_trace: &bool) -> Result<HdnSuccess, HdnError> {
    use crate::HdnError::*;
    use crate::HdnSuccess::*;

    let file = get_home_dot_nix().map_err(CouldNotFindHomeDotNix)?;

    let content = fs::read_to_string(&file).map_err(CouldNotReadFile)?;

    let new_content = update_nix(&content, packages, &mode)
        .map_err(CouldNotUpdatePackages)?;

    if new_content.eq(&content) {
        return match mode {
            UpdateNixMode::Add => Ok(NothingToAdd),
            UpdateNixMode::Remove => Ok(NothingToRemove)
        };
    }

    diff::print_diff(&content, &new_content);
    println!();

    fs::write(&file, new_content).map_err(CouldNotWriteToFile)?;

    let run_result = run_home_manager_switch(show_trace);
    if let Err(error) = run_result {
        // Skip printing the error if home-manager returned a non-zero exit code,
        // since home-manager prints its own errors.
        if !matches!(error, RunHomeManagerSwitchError::Unsuccessful) {
            print_error(error);
        }
        println!();

        fs::write(&file, content)
            .map_err(UnsuccessfulAndNotRolledBack)?;

        return Ok(HomeManagerSwitchErroredButRollbackSuccessful);
    }
    println!();
    Ok(HomeManagerSwitchSucceeded)
}

fn add(packages: &Vec<String>, show_trace: &bool) -> Result<HdnSuccess, HdnError> {
    update(UpdateNixMode::Add, packages, show_trace)
}

fn remove(packages: &Vec<String>, show_trace: &bool) -> Result<HdnSuccess, HdnError> {
    update(UpdateNixMode::Remove, packages, show_trace)
}

fn print_error<T: Error>(error: T) {
    let error_prefix = "error:".red().bold().to_string();
    eprintln!("{error_prefix} {}", error);

    fn print_sources<T: Error>(error: T) {
        if let Some(source) = error.source() {
            eprintln!("caused by: {}", source);
            print_sources(source);
        }
    }
    print_sources(error);
}

fn main() -> ExitCode {
    let cli = HdnCli::parse();

    let result = match &cli.subcommand {
        HdnSubcommand::Add {packages, show_trace} => {
            add(packages, show_trace)
        }

        HdnSubcommand::Remove { packages, show_trace} => {
            remove(packages, show_trace)
        }
    };

    match result {
        Err(error) => {
            print_error(error);
            ExitCode::FAILURE
        }
        Ok(success) => {
            println!("{success}");
            ExitCode::SUCCESS
        }
    }
}
