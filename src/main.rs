use std::{fs};
use std::collections::HashSet;
use std::process::{Command};
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

fn add(packages: &Vec<String>, show_trace: &bool) {
    let file = dirs::home_dir()
        .expect("Home directory should exist")
        .join(".config/home-manager/home.nix");

    let fs_read_result = fs::read_to_string(file.clone());
    let content = match fs_read_result {
        Ok(content) => content,
        Err(error) => {
            print_error(format!("could not open home.nix: {error}"));
            return;
        }
    };

    let nix_read_result = nix_editor::read::getarrvals(&content, QUERY);
    let existing_packages: HashSet<String> = match nix_read_result {
        Ok(vec) => HashSet::from_iter(vec),
        Err(_error) => {
            print_error(format!("could not get values of {QUERY} attribute in home.nix"));
            return;
        }
    };

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
        return;
    }

    println!("{}", format!("Adding {:?} to home.nix", packages_to_add).bold());

    let nix_add_result = nix_editor::write::addtoarr(&content, QUERY, packages_to_add.into_iter().cloned().collect());
    let new_content = match nix_add_result {
        Ok(new_content) => new_content,
        Err(_error) => {
            print_error(format!("could not update {QUERY} attribute for new packages"));
            return;
        }
    };

    match fs::write(file.clone(), new_content) {
        Ok(..) => {}
        Err(error) => {
            print_error(format!("could not write to home.nix: {error}"));
            return;
        }
    }

    let mut command = Command::new("home-manager");
    let command = command.arg("switch");
    let command = if *show_trace {command.arg("--show-trace")} else {command};

    let mut child = command.spawn()
        .expect("Should able to run home-manager switch");
    println!("Running home-manager switch: PID {}", child.id());

    if child.wait().unwrap().success() {
        println!("{}", "Successfully updated home.nix and activated generation".bold());
    } else {
        println!("Running home-manager switch resulted in an error, reverting home.nix");

        match fs::write(file, content) {
            Ok(..) => {}
            Err(error) => {
                print_error(format!("could not write to home.nix: {error}"));
                #[allow(clippy::needless_return)] return;
            }
        };
    }
}

fn main() {
    let cli = HdnCli::parse();

    match &cli.subcommand {
        HdnSubcommand::Add {packages, show_trace} => {
            add(packages, show_trace);
        }

        HdnSubcommand::Remove { packages: _packages } => {
            print_error(String::from("the remove command isn't implemented yet"));
        }
    }
}
