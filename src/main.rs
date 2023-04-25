use std::{env, fs};
use std::ptr::write;

fn main() {
    let content = fs::read_to_string("/Users/fisher/Desktop/home.nix").unwrap();
    let packages = nix_editor::read::getarrvals(&content, "home.packages").unwrap();

    let args: Vec<String> = env::args().collect();

    let mut to_add= Vec::new();
    for arg in &args[1..] {
        if packages.contains(&arg) {
            println!("home.nix already contains {}, skipping it", arg);
        } else {
            to_add.push(arg);
        }
    }

    println!("adding {:?} to home.nix", to_add);

    let result = nix_editor::write::addtoarr(&content, "home.packages", to_add.into_iter().cloned().collect());
    println!("{}", result.unwrap());
}
