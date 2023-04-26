use std::{env, fs, thread};
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use owo_colors::{OwoColorize};

const QUERY: &str = "home.packages";

fn print_error(message: String) {
    let error_prefix = "Error:".red().bold().to_string();
    eprintln!("{error_prefix} {message}");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] != "add" {
        print_error(String::from("Only the \"add\" subcommand is supported currently"));
        return;
    }

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
            print_error(format!("Could not get values of {QUERY} attribute in home.nix"));
            return;
        }
    };

    let mut packages_to_add = Vec::new();
    for arg in args.iter().skip(2) {
        if existing_packages.contains(arg) {
            println!("Skipping \"{arg}\": already in home.nix");
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
            print_error(format!("Could not update {QUERY} attribute for new packages"));
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

    // https://stackoverflow.com/a/49063262
    let mut child = Command::new("home-manager")
        .arg("switch")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Should able to run home-manager switch");
    println!("Running home-manager switch: PID {}", child.id());

    let stdout = BufReader::new(child.stdout.take().expect("Child process should have stdout"));
    let stderr = BufReader::new(child.stderr.take().expect("Child process should have stderr"));

    let thread = thread::spawn(move || {
        stderr.lines().for_each(
            |line| println!("{}", line.unwrap())
        );
    });
    stdout.lines().for_each(
        |line| println!("{}", line.unwrap())
    );
    thread.join().unwrap();

    if child.wait().unwrap().success() {
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
