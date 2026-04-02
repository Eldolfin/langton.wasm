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
              git-hooks.package = pkgs.prek;
              # formatters
              alejandra.enable = true; # nix
              end-of-file-fixer.enable = true;
              rustfmt.enable = true;
              trim-trailing-whitespace.enable = true;
              statix.enable = true;
              check-merge-conflicts.enable = true;
              check-yaml.enable = true;
              markdownlint.enable = true;
              clippy = {
                enable = true;
                settings = {
                  allFeatures = true;
                  denyWarnings = true;
                };
              };
            };
          };
        };

        devShell = pkgs.mkShell {
          inherit (self.checks.${system}.pre-commit-check) shellHook;
          RUSTC = "${pkgs.rustc}/bin/rustc";
          buildInputs = with pkgs;
            [
              cargo
              entr
              just
              live-server
              lld
              nodejs
              prek
              rust-analyzer
              rustc
              wasm-pack
            ]
            ++ self.checks.${system}.pre-commit-check.enabledPackages;
        };
      }
    );
}
