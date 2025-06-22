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
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, pre-commit-hooks }:
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
            # Simple formatting checks
            rustfmt.enable = true;
            nixpkgs-fmt.enable = true;
          };
        };
      in
      {
        formatter = pkgs.treefmt;

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

        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Node
            nodejs_22
            nodePackages.eslint
            nodePackages.npm
            nodePackages.prettier
            nodePackages.typescript
            nodePackages.typescript-language-server

            # Rust
            rustToolchain
            wasm-pack
            wasm-bindgen-cli
            binaryen # for wasm-opt

            # Treefmt
            treefmt
            nixpkgs-fmt

            git
            direnv
          ] ++ pre-commit-check.enabledPackages;

          shellHook = pre-commit-check.shellHook + ''
            echo ""
            echo "Vite Plugin Norg Development Environment"
            echo "Node.js: $(node --version)"
            echo "Rust: $(rustc --version)"
            echo "wasm-pack: $(wasm-pack --version)"
            echo ""
            echo "Available commands:"
            echo "  npm run build      - Build everything"
            echo "  npm run build:wasm - Build WASM parser"  
            echo "  npm run build:js   - Build TypeScript"
            echo "  npm test           - Run tests"
            echo "  npm run lint       - Run linter"
            echo "  npm run type-check - Run TypeScript type check"
            echo "  cargo check        - Check Rust code"
            echo "  nix fmt            - Format all code with treefmt"
            echo "  nix flake check    - Run all checks via pre-commit"
            echo "  pre-commit run -a  - Run all hooks manually"
            echo ""
          '';
        };
      });
}

