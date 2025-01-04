{inputs, ...}: {
  perSystem = {
    config,
    pkgs,
    system,
    inputs',
    self',
    ...
  }:
    with inputs; let
      inherit (self'.packages) rust-toolchain;
      inherit (self'.legacyPackages) cargoExtraPackages ciPackages;

      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
          (final: prev: {
            lpi = inputs.lpi.packages.${system}.default;
          })
        ];
      };

      devTools = with pkgs; [
        # rust tooling
        rust-toolchain
        cargo-audit
        cargo-udeps
        cargo-limit
        bacon
        cargo-watch
        cargo-audit
        cargo-deny
        # cargo-llvm-cov
        cargo-tarpaulin
        cargo-nextest
        cargo-outdated
        # formatting
        self'.packages.treefmt
        taplo
        # misc
        lpi
        # logging
        bunyan-rs.out
        # command runner
        just
        nushell
      ];
    in {
      devShells = {
        default = pkgs.mkShell {
          name = "Seeking-Edge-shell";
          RUST_SRC_PATH = "${self'.packages.rust-toolchain}/lib/rustlib/src/rust/src";
          AMD_VULKAN_ICD = "RADV";

          LD_LIBRARY_PATH = "$LD_LIBRARY_PATH:${with pkgs;
            lib.makeLibraryPath [
              udev
              alsa-lib
              vulkan-loader
              libxkbcommon
              openssl
              wayland # To use wayland feature
            ]}";
          packages = devTools ++ cargoExtraPackages ++ ciPackages;

          shellHook = ''
            # cargo install puffin_viewer -q
            # cargo install cargo-machete -q
            # cargo install cargo-nextest-q
            export EDITOR=hx
            # zellij session
            alias zj="zellij --layout dev-layout.kdl"

            SESSION="seeking-edge-dev"
            ZJ_SESSIONS=$(zellij list-sessions -n | rg 'seeking-edge-dev' ) #$SESSION )

             if [[ $ZJ_SESSIONS == *"seeking-edge-dev"* ]]; then
               # exec zellij attach seeking-edge-dev options --default-layout ./dev-layout.kdl
             else
               # exec zellij --session seeking-edge-dev --layout ./dev-layout.kdl
             fi
          '';
        };
      };
    };
}
