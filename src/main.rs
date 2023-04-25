use std::{env, fs};
use std::ptr::write;

fn main() {
    let content = fs::read_to_string("/Users/fisher/Desktop/home.nix").unwrap();
    let packages = nix_editor::read::getarrvals(&content, "home.packages").unwrap();

    let args: Vec<String> = env::args().collect();
    let item = &args[1];

    if packages.contains(item) {
        println!("home.nix already contains this package. stopping.");
        return;
    }

    println!("adding {} to home.nix", item);
    let result = nix_editor::write::addtoarr(&content, "home.packages", vec![item].into_iter().cloned().collect());
    println!("{}", result.unwrap());
}
