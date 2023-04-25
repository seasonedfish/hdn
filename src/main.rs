use std::{env, fs};
use std::collections::HashSet;

const FILE: &str = "/Users/fisher/Desktop/home.nix";
const QUERY: &str = "home.packages";

fn main() {
    let read_file_result = fs::read_to_string(FILE);
    let content = match read_file_result {
        Ok(content) => content,
        Err(error) => {
            eprintln!("Could not open home.nix: {}", error);
            return;
        }
    };

    let packages: HashSet<String> = HashSet::from_iter(nix_editor::read::getarrvals(&content, QUERY).unwrap());

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
    fs::write(FILE, result).expect("Failed to write file");
}
