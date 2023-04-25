use std::{env, fs};
use std::collections::HashSet;
use std::process::Command;
use owo_colors::{OwoColorize};

const QUERY: &str = "home.packages";

fn print_error(message: String) {
    let error_prefix = "Error:".red().bold().to_string();
    eprintln!("{error_prefix} {message}");
}

fn main() {
    let file = dirs::home_dir()
        .expect("Home directory should exist")
        .join(".config/home-manager/home.nix");

    println!("Using {} as home.nix", file.display());

    let fs_read_result = fs::read_to_string(file.clone());
    let content = match fs_read_result {
        Ok(content) => content,
        Err(error) => {
            print_error(format!("Could not open home.nix: {error}"));
            return;
        }
    };
    let nix_read_result = nix_editor::read::getarrvals(&content, QUERY);
    let existing_packages: HashSet<String> = match nix_read_result {
        Ok(vec) => HashSet::from_iter(vec),
        Err(_error) => {
            print_error(format!("Could not get values of {QUERY} in home.nix"));
            return;
        }
    };

    let args: Vec<String> = env::args().collect();

    let mut packages_to_add = Vec::new();
    for arg in args.iter().skip(1) {
        if existing_packages.contains(arg) {
            println!("Skipping {}: already in home.nix", arg);
        } else {
            packages_to_add.push(arg);
        }
    }

    if packages_to_add.len() == 0 {
        println!("Nothing to add to home.nix");
        return;
    }

    println!("Adding {:?} to home.nix", packages_to_add);

    let nix_add_result = nix_editor::write::addtoarr(&content, QUERY, packages_to_add.into_iter().cloned().collect());
    let new_content = match nix_add_result {
        Ok(new_content) => new_content,
        Err(_error) => {
            print_error(format!("Could not update nix for new packages"));
            return;
        }
    };
    match fs::write(file.clone(), new_content) {
        Ok(..) => {}
        Err(error) => {
            print_error(format!("Could not write to home.nix: {error}"));
            return;
        }
    }

    let output = match Command::new("home-manager").arg("switch").output() {
        Ok(output) => output,
        Err(error) => {
            print_error(format!("Could not run home-manager switch: {error}"));
            return;
        }
    };

    print!("{}", String::from_utf8(output.stdout).unwrap());

    if output.status.success() {
        println!("Successfully added packages and activated new generation");
    } else {
        println!("Running home-manager switch resulted in an error, reverting home.nix");

        match fs::write(file, content) {
            Ok(..) => {}
            Err(error) => {
                print_error(format!("Could not write to home.nix: {error}"));
                return;
            }
        };
    }
}
