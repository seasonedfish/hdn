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

    let read_file_result = fs::read_to_string(FILE);
    let content = match read_file_result {
        Ok(content) => content,
        Err(error) => {
            print_error(format!("Could not open home.nix: {error}"));
            return;
        }
    };
    let nix_read_result = nix_editor::read::getarrvals(&content, QUERY);
    let packages: HashSet<String> = match nix_read_result {
        Ok(vec) => HashSet::from_iter(vec),
        Err(_error) => {
            print_error(format!("Could not get values of {QUERY} in home.nix"));
            return;
        }
    };

    let args: Vec<String> = env::args().collect();

    let mut to_add= Vec::new();
    for arg in &args[1..] {
        if packages.contains(arg) {
            println!("Skipping {}: already in home.nix", arg);
        } else {
            to_add.push(arg);
        }
    }

    if to_add.len() == 0 {
        println!("Nothing to add to home.nix");
        return;
    }

    println!("Adding {:?} to home.nix", to_add);

    let result = nix_editor::write::addtoarr(&content, QUERY, to_add.into_iter().cloned().collect()).unwrap();
    match fs::write(FILE, result) {
        Ok(..) => {}
        Err(error) => {
            print_error(format!("Could not add to home.nix: {error}"));
            return;
        }
    }
}
