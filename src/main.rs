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
    Remove {packages: Vec<String>}
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
        eprintln!("caused by: error: {}", error);

        if let Some(source) = error.source() {
            print_sources(source);
        }
    }
    if let Some(source) = error.source() {
        print_sources(source);
    }
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

enum UpdatePackagesMode {
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

fn update_packages(content: &String, packages: &Vec<String>, mode: UpdatePackagesMode) -> Result<String, UpdatePackagesError> {
    use crate::UpdatePackagesError::{CouldNotReadNix, CouldNotWriteNix};
    use crate::UpdatePackagesMode::{Add, Remove};

    let packages: IndexSet<&String> = IndexSet::from_iter(packages);

    let existing_packages: IndexSet<String> = IndexSet::from_iter(
        nix_editor::read::getarrvals(&content, QUERY)
            .map_err(CouldNotReadNix)?
    );

    return match mode {
        Add => {
            let transformed_packages: Vec<&String> = packages.into_iter()
                .filter(|&p| !existing_packages.contains(p))
                .collect();

            nix_editor::write::addtoarr(
                &content,
                QUERY,
                transformed_packages.into_iter().cloned().collect()
            ).map_err(CouldNotWriteNix)
        }
        Remove => {
            let transformed_packages: Vec<&String> = packages.into_iter()
                .filter(|&p| existing_packages.contains(p))
                .collect();

            nix_editor::write::rmarr(
                &content,
                QUERY,
                transformed_packages.into_iter().cloned().collect()
            ).map_err(CouldNotWriteNix)
        }
    }
}

#[derive(Error, Debug)]
enum AddError {
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

fn add(packages: &Vec<String>, show_trace: &bool) -> Result<(), AddError> {
    use crate::AddError::{CouldNotReadFile, CouldNotWriteToFile, UnsuccessfulAndNotRolledBack, UnsuccessfulButRolledBack, CouldNotUpdatePackages, NothingToUpdate};

    let file = dirs::home_dir()
        .expect("Home directory should exist")
        .join(".config/home-manager/home.nix");

    let content = fs::read_to_string(file.clone()).map_err(CouldNotReadFile)?;

    let new_content = update_packages(&content, packages, UpdatePackagesMode::Add)
        .map_err(CouldNotUpdatePackages)?;

    if new_content.eq(&content) {
        return Err(NothingToUpdate);
    }

    fs::write(file.clone(), new_content).map_err(CouldNotWriteToFile)?;

    match run_home_manager_switch(show_trace) {
        Ok(()) => {
            println!("{}", "Successfully updated home.nix and activated generation".bold());
            Ok(())
        }
        Err(error) => {
            fs::write(file, content)
                .map_err(UnsuccessfulAndNotRolledBack)?;

            Err(UnsuccessfulButRolledBack(error))
        }
    }
}

#[derive(Error, Debug)]
#[error("this command isn't implemented yet")]
struct NotImplementedError;

fn main() {
    let cli = HdnCli::parse();

    match &cli.subcommand {
        HdnSubcommand::Add {packages, show_trace} => {
            if let Err(error) = add(packages, show_trace) {
                print_error(error);
            }
        }

        HdnSubcommand::Remove { packages: _packages } => {
            print_error(NotImplementedError);
        }
    }
}
