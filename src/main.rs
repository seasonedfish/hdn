use std::{fs};
use std::collections::HashSet;
use std::process::{Command};
use anyhow::{anyhow, Context};
use owo_colors::{OwoColorize};
use clap::{Parser, Subcommand};

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

fn print_error(message: String) {
    let error_prefix = "error:".red().bold().to_string();
    eprintln!("{error_prefix} {message}");
}

fn run_home_manager_switch(show_trace: &bool) -> Result<(), anyhow::Error> {
    let mut command = Command::new("home-manager");
    let command = command.arg("switch");
    let command = if *show_trace {command.arg("--show-trace")} else {command};

    let mut child = command
        .spawn()?;
    println!("Running home-manager switch: PID {}", child.id());

    let exit_status = child.wait()?;

    if !exit_status.success() {
        return Err(anyhow!("home-manager switch returned non-zero exit code"));
    }
    Ok(())
}

fn add(packages: &Vec<String>, show_trace: &bool) -> Result<(), anyhow::Error> {
    let file = dirs::home_dir()
        .expect("Home directory should exist")
        .join(".config/home-manager/home.nix");

    let content = fs::read_to_string(file.clone())
        .with_context(|| "could not open home.nix")?;

    let existing_packages: HashSet<String> = HashSet::from_iter(
        nix_editor::read::getarrvals(&content, QUERY)
            .with_context(|| format!("could not get values of {QUERY} attribute in home.nix"))?
    );

    let mut packages_to_add = Vec::new();
    for package in packages {
        if existing_packages.contains(package) {
            println!("Skipping \"{package}\": already in home.nix");
        } else {
            packages_to_add.push(package);
        }
    }

    if packages_to_add.is_empty() {
        println!("Nothing to add to home.nix");
        return Ok(());
    }

    println!("{}", format!("Adding {:?} to home.nix", packages_to_add).bold());

    let new_content =  nix_editor::write::addtoarr(
        &content,
        QUERY,
        packages_to_add.into_iter().cloned().collect()
    ).with_context(|| format!("could not update {QUERY} attribute for new packages"))?;


    fs::write(file.clone(), new_content).with_context(|| "could not write to home.nix:")?;

    match run_home_manager_switch(show_trace) {
        Ok(()) => {
            println!("{}", "Successfully updated home.nix and activated generation".bold());
            Ok(())
        }
        Err(..) => {
            fs::write(file, content)
                .with_context(|| "Running home-manager switch resulted in an error. During the rollback of home.nix, another error occurred.")?;

            Err(anyhow!("Running home-manager switch resulted in an error. Your home.nix has been rolled back."))
        }
    }
}

fn main() {
    let cli = HdnCli::parse();

    match &cli.subcommand {
        HdnSubcommand::Add {packages, show_trace} => {
            if let Err(error) = add(packages, show_trace) {
                print_error(error.to_string());
            }
        }

        HdnSubcommand::Remove { packages: _packages } => {
            print_error(String::from("the remove command isn't implemented yet"));
        }
    }
}
