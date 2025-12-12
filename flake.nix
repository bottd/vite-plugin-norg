{
  description = "Vite plugin for processing Norg files";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, pre-commit-hooks, treefmt-nix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common args for crane
        commonArgs = {
          src = ./.;
          buildInputs = with pkgs; [
            # Add any system dependencies here
          ];
        };

        # Build dependencies first for reuse
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;


        pre-commit-check = pre-commit-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            nixpkgs-fmt.enable = true;
          };
        };

        treefmtEval = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs = {
            prettier = {
              enable = true;
              includes = [ "*.js" "*.ts" "*.json" "*.md" "*.yaml" "*.yml" ];
            };
            rustfmt = {
              enable = true;
              package = rustToolchain;
            };
            nixpkgs-fmt.enable = true;
          };
        };
      in
      {
        formatter = treefmtEval.config.build.wrapper;

        checks = {
          pre-commit-check = pre-commit-check;

          # Crane-based Rust checks that handle dependencies properly
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "-- --deny warnings";
          });

          rust-tests = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });

          formatting = treefmtEval.config.build.check self;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Node
            nodejs_24
            bun

            # Rust
            rustToolchain
            wasm-pack
            wasm-bindgen-cli
            binaryen # for wasm-opt

            # Formatters
            treefmtEval.config.build.wrapper
            nixpkgs-fmt

            git
            direnv
          ] ++ pre-commit-check.enabledPackages;

          shellHook = pre-commit-check.shellHook;
        };
      });
}

