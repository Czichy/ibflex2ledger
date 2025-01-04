{
  description = "A rust project";

  inputs = {
    nixpkgs = {
      url = "github:NixOS/nixpkgs/nixos-unstable";
    };
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs = {
          follows = "nixpkgs";
        };
      };
    };
    crane = {
      url = "github:ipetkov/crane";
    };
    nix-filter = {
      url = "github:numtide/nix-filter";
    };
    process-compose-flake = {
      url = "github:Platonic-Systems/process-compose-flake";
    };
    services-flake = {
      url = "github:juspay/services-flake";
    };
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
    bomper = {
      url = "github:justinrubek/bomper";
    };
    lpi = {
      url = "github:cymenix/lpi";
    };
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux"];
      imports = [
        inputs.flake-parts.flakeModules.easyOverlay

        inputs.bomper.flakeModules.bomper
        inputs.pre-commit-hooks.flakeModule

        ./flake-parts/cargo.nix
        ./flake-parts/rust-toolchain.nix

        ./flake-parts/pre-commit.nix
        ./flake-parts/formatting.nix

        ./flake-parts/bomper.nix
        ./flake-parts/shells.nix
        ./flake-parts/ci.nix
      ];
    };
}
