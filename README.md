# hdn: utility for updating home.nix
Home Manager is great,
but it's tedious to run `home-manager edit`,
add your package to the file,
and run `home-manager switch`
every time you want to install a package.

I wanted a workflow like Poetry,
where I could just add a package and install it in one command.
So, I made `hdn`.

You can call it like so:
```shell
hdn add pkgs.hello
```

This will add `pkgs.hello` to your `home.packages` in `home.nix`, and call `home-manager switch`.

If `home-manager switch` fails, it will automatically roll back `home.nix` to its previous version. 

## Requirements
This program requires that:
- you have `home-manger` on your PATH
- `home.nix` lives in `~/.config/home-manager/`
- `home.nix` contains the attribute `home.packages`, the list of packages in the user environment

These requirements should be satisfied with the default home-manager installation.

## Disclaimer
This program uses Rust, and I don't actually know how to program in Rust.

(I chose Rust because I found a Rust library for easily reading and writing nix files.)

Someday I'll go back, actually learn Rust, and rewrite this, but for now, use at your own risk.

## Todos
- Support `~/.config/nixpkgs/` location for home.nix
- Support "remove" subcommand
- Add "--help" option
- Organize code into functions
- Use rnix directly instead of `nix-editor`
- Show `home.nix` diff
- Actually learn Rust
