{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils = {
      url = "github:numtide/flake-utils";
    };
    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {
    self,
    nixpkgs,
    utils,
    pre-commit-hooks,
  }:
    utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        checks = {
          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              # formatters
              alejandra.enable = true; # nix
              denofmt.enable = true; # markdown
              end-of-file-fixer.enable = true;
              rustfmt.enable = true;
              trim-trailing-whitespace.enable = true;

              # linters
              actionlint.enable = true;
              statix.enable = true;
              check-added-large-files.enable = true;
              check-merge-conflicts.enable = true;
              check-yaml.enable = true;
              markdownlint.enable = true;
              cargo-clippy = {
                enable = true;
                name = "cargo-clippy";
                description = "Check the cargo package for errors with clippy";
                entry = "${pkgs.cargo}/bin/cargo clippy -- -Dwarnings";
                files = "\\.rs$";
                pass_filenames = false;
              };
            };
          };
        };

        devShell = pkgs.mkShell {
          inherit (self.checks.${system}.pre-commit-check) shellHook;
          buildInputs = with pkgs; [
            wasm-pack
            lld
            bacon
          ];
        };
      }
    );
}
