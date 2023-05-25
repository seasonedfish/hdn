# hdn: utility for updating home.nix
Home Manager is great,
but it's tedious to run `home-manager edit`,
add your package to the file,
and run `home-manager switch`
every time you want to install a package.

I wanted a workflow like Poetry,
where I could just add a package and install it in one command.
So, I made `hdn`.

You can call it with e.g. `hdn add pkgs.hello pkgs.cowsay`:

<img width="794" alt="image" src="https://github.com/seasonedfish/hdn/assets/29507110/0a6fa19b-34c0-4246-9d4e-41b114927d13">

This adds `pkgs.hello` and `pkgs.cowsay` to the `home.packages` attribute in `home.nix`, and calls `home-manager switch`.

If `home-manager switch` fails, it will automatically roll back `home.nix` to its original state. 

## Requirements
This program requires that:
- you have `home-manger` on your PATH
- `home.nix` lives in one of the default locations (namely, `~/.config/home-manager/`, `~/.config/nixpkgs/`, `~/.nixpkgs/`)
- `home.nix` contains the attribute `home.packages`, the list of packages in the user environment

These requirements should be satisfied with the default home-manager installation.

## Disclaimer
This program uses Rust, and I don't actually know how to program in Rust.

(I chose Rust because I found a Rust library for easily reading and writing nix files.)

Someday I'll go back, actually learn Rust, and rewrite this, but for now, use at your own risk.

## Installation
Releases are available on crates.io.
```shell
cargo install hdn
```

## Acknowledgements
This project was made possible by the work of others (that I ~~stole~~ legally incorporated).

I thank Victor Fuentes for his work on [nix-editor](https://github.com/vlinkz/nix-editor);
the code for nix parsing and writing comes from his project.

I thank Armin Ronacher for his work on [similar](https://github.com/mitsuhiko/similar);
the code that displays the `home.nix` diff comes from his project.
