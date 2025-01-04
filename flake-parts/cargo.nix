{inputs, ...}: {
  perSystem = {
    config,
    # pkgs,
    system,
    inputs',
    self',
    lib,
    ...
  }:
    with inputs; let
      manifest = (pkgs.lib.importTOML ../Cargo.toml).package;
      # rustToolchain = fenix.packages.${system}.fromToolchainFile {
      #   file = ../rust-toolchain.toml;
      #   sha256 = "sha256-pZJWdNhvEsGbBM5yMD3xGi5IaGb01eyRvhCqUVAtFU8=";
      # };
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
          (final: prev: {
            lpi = inputs.lpi.packages.${system}.default;
          })
        ];
      };
      fenix-channel = fenix.packages.${system}.latest;

      fenix-toolchain = fenix-channel.withComponents [
        "rustc"
        "cargo"
        "clippy"
        "rust-src"
        "llvm-tools-preview"
      ];

      craneLib = (crane.mkLib pkgs).overrideToolchain fenix-toolchain;

      src = nix-filter.lib {
        root = ../.;
        include = [
          "Cargo.toml"
          "Cargo.lock"
          "taplo.toml"
          "rustfmt.toml"
          "rust-toolchain.toml"
          "crates/api"
          "crates/flex"
        ];
      };

      inherit (craneLib.crateNameFromCargoToml {inherit src;}) pname version;

      args = {
        inherit src;
        strictDeps = true;
        nativeBuildInputs = with pkgs; [
          openssl
          pkg-config
          # udev
        ];
        buildInputs = with pkgs; [
          openssl.dev
          openssl
          pkg-config
        ];
        LD_LIBRARY_PATH = lib.makeLibraryPath [pkgs.openssl];
        # Needed to get openssl-sys to use pkg-config.
        # Doesn't seem to like OpenSSL 3
        OPENSSL_NO_VENDOR = 1;
        PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib/";
      };

      individualCrateArgs =
        args
        // {
          inherit cargoArtifacts version;
          doCheck = false;
        };

      fileSetForCrate = crateFiles:
        nix-filter.lib {
          root = ../.;
          include =
            [
              "crates"
              "Cargo.toml"
              "Cargo.lock"
            ]
            ++ crateFiles;
        };

      cargoArtifacts = craneLib.buildDepsOnly args;

      api = craneLib.buildPackage (individualCrateArgs
        // rec {
          pname = manifest.name;
          version = manifest.version;
          cargoExtraArgs = "--lib ${pname}";
          src = fileSetForCrate [
            "crates/api/src"
            "crates/api/Cargo.toml"
          ];
        });

      flex = craneLib.buildPackage (individualCrateArgs
        // rec {
          pname = "ibkr-rust-flex";
          cargoExtraArgs = "--bin ${pname}";
          src = fileSetForCrate [
            "crates/flex/src"
            "crates/flex/Cargo.toml"
          ];
        });
    in {
      checks = {
        inherit seeking-edge;
        # inherit app api server kickbase assets kickbase-api-doc;
        inherit (self.packages.${system}) services;

        clippy = craneLib.cargoClippy (args
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        doc = craneLib.cargoDoc (args
          // {
            inherit cargoArtifacts;
          });

        fmt = craneLib.cargoFmt {
          inherit src;
        };

        toml-fmt = craneLib.taploFmt {
          src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];
          taploExtraArgs = "--config ../taplo.toml";
        };

        audit = craneLib.cargoAudit {
          inherit src advisory-db;
        };

        deny = craneLib.cargoDeny {
          inherit src;
        };

        nextest = craneLib.cargoNextest (args
          // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });
      };

      packages = {
        inherit flex api;
        inherit (self.checks.${system}) coverage;
        default = self.packages.${system}.flex;
      };
      legacyPackages = {
        cargoExtraPackages = args.nativeBuildInputs;
      };

      apps = {
        default = {
          program = self.packages.${system}.flex;
        };
      };

      formatter = pkgs.alejandra;
    };
}
