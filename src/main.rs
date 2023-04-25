use std::{env, fs};
use std::collections::HashSet;
use owo_colors::{OwoColorize};

const FILE: &str = "/Users/fisher/Desktop/home.nix";
const QUERY: &str = "home.packages";

fn print_error(message: String) {
    let error_prefix = "Error:".red().bold().to_string();
    eprintln!("{error_prefix} {message}");
}

fn main() {
    println!("Using {FILE} as home.nix");

    let fs_read_result = fs::read_to_string(FILE);
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
    match fs::write(FILE, new_content) {
        Ok(..) => {}
        Err(error) => {
            print_error(format!("Could not write to home.nix: {error}"));
            return;
        }
    }
}
