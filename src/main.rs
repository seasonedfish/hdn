use std::{env, fs};
use std::collections::HashSet;
use std::ptr::write;

const FILE: &str = "/Users/fisher/Desktop/home.nix";
const QUERY: &str = "home.packages";

fn main() {
    let content = fs::read_to_string(FILE).unwrap();
    let packages: HashSet<String> = HashSet::from_iter(nix_editor::read::getarrvals(&content, QUERY).unwrap());

    let args: Vec<String> = env::args().collect();

    let mut to_add= Vec::new();
    for arg in &args[1..] {
        if packages.contains(arg) {
            println!("home.nix already contains {}, skipping it", arg);
        } else {
            to_add.push(arg);
        }
    }

    println!("adding {:?} to home.nix", to_add);

    let result = nix_editor::write::addtoarr(&content, QUERY, to_add.into_iter().cloned().collect()).unwrap();
    fs::write(FILE, result).expect("Failed to write file");
}
