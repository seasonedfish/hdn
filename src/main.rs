pub mod diff;

use std::{fs, io};
use std::error::Error;
use std::process::{Command};
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
    use crate::RunHomeManagerSwitchError::{CouldNotRun, OSError, Unsuccessful};

    let mut command = Command::new("home-manager");
    let command = command.arg("switch");
    let command = if *show_trace {command.arg("--show-trace")} else {command};

    let mut child = command
        .spawn()
        .map_err(CouldNotRun)?;
    println!("Running home-manager switch: PID {}", child.id());

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
enum UpdatePackagesError {
    #[error("could not read values of home.packages attribute in home.nix")]
    CouldNotReadNix(#[source] nix_editor::read::ReadError),
    #[error("could not write home.packages attribute for new packages")]
    CouldNotWriteNix(#[source] nix_editor::write::WriteError),
}

fn update_nix(content: &str, packages: &Vec<String>, mode: UpdateNixMode) -> Result<String, UpdatePackagesError> {
    use crate::UpdatePackagesError::{CouldNotReadNix, CouldNotWriteNix};
    use crate::UpdateNixMode::{Add, Remove};

    let packages: IndexSet<&String> = IndexSet::from_iter(packages);

    let existing_packages: IndexSet<String> = IndexSet::from_iter(
        nix_editor::read::getarrvals(content, QUERY)
            .map_err(CouldNotReadNix)?
    );

    match mode {
        Add => {
            let transformed_packages: Vec<&String> = packages.into_iter()
                .filter(|&p| !existing_packages.contains(p))
                .collect();

            nix_editor::write::addtoarr(
                content,
                QUERY,
                transformed_packages.into_iter().cloned().collect()
            ).map_err(CouldNotWriteNix)
        }
        Remove => {
            let transformed_packages: Vec<&String> = packages.into_iter()
                .filter(|&p| existing_packages.contains(p))
                .collect();

            nix_editor::write::rmarr(
                content,
                QUERY,
                transformed_packages.into_iter().cloned().collect()
            ).map_err(CouldNotWriteNix)
        }
    }
}

#[derive(Error, Debug)]
enum HdnError {
    #[error("could not read home.nix")]
    CouldNotReadFile(#[source] io::Error),
    #[error("could not write to home.nix")]
    CouldNotWriteToFile(#[source] io::Error),
    #[error("running home-manager switch errored, and during the rollback of home.nix, another error occurred")]
    UnsuccessfulAndNotRolledBack(#[source] io::Error),
    #[error("running home-manager switch errored; your home.nix has been rolled back")]
    UnsuccessfulButRolledBack(#[source] RunHomeManagerSwitchError),
    #[error("could not update home.packages attribute in home.nix")]
    CouldNotUpdatePackages(#[source] UpdatePackagesError),
    #[error("nothing to update in home.nix, home-manager switch was not run")]
    NothingToUpdate
}

fn update(mode: UpdateNixMode, packages: &Vec<String>, show_trace: &bool) -> Result<(), HdnError> {
    use crate::HdnError::{CouldNotReadFile, CouldNotWriteToFile, UnsuccessfulAndNotRolledBack, UnsuccessfulButRolledBack, CouldNotUpdatePackages, NothingToUpdate};

    let file = dirs::home_dir()
        .expect("Home directory should exist")
        .join(".config/home-manager/home.nix");

    let content = fs::read_to_string(file.clone()).map_err(CouldNotReadFile)?;

    let new_content = update_nix(&content, packages, mode)
        .map_err(CouldNotUpdatePackages)?;

    if new_content.eq(&content) {
        return Err(NothingToUpdate);
    }

    diff::print_diff(&content, &new_content);
    println!();

    fs::write(file.clone(), new_content).map_err(CouldNotWriteToFile)?;

    if let Err(error) = run_home_manager_switch(show_trace) {
        fs::write(file, content)
            .map_err(UnsuccessfulAndNotRolledBack)?;

        return Err(UnsuccessfulButRolledBack(error));
    }
    Ok(())
}

fn add(packages: &Vec<String>, show_trace: &bool) -> Result<(), HdnError> {
    update(UpdateNixMode::Add, packages, show_trace)
}

fn remove(packages: &Vec<String>, show_trace: &bool) -> Result<(), HdnError> {
    update(UpdateNixMode::Remove, packages, show_trace)
}

fn main() {
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
        }
        Ok(()) => {
            println!("{}", "Successfully updated home.nix and activated generation".bold());
        }
    };
}
