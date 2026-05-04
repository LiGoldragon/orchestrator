{
  description = "Criopolis cascade orchestrator daemon.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";

    crane.url = "github:ipetkov/crane";

    gascity-nix.url = "github:LiGoldragon/gascity-nix";
    gascity-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      crane,
      gascity-nix,
    }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forSystems = function: nixpkgs.lib.genAttrs systems (system: function system);

      mkContext =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ gascity-nix.overlays.default ];
          };
          toolchain = fenix.packages.${system}.stable.withComponents [
            "cargo"
            "rustc"
            "rustfmt"
            "clippy"
            "rust-analyzer"
            "rust-src"
          ];
          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
          src = craneLib.cleanCargoSource ./.;
          commonArgs = {
            inherit src;
            strictDeps = true;
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          gascityPackage = pkgs.gascity;
          beadsPackage = pkgs.beads;
        in
        {
          inherit
            pkgs
            toolchain
            craneLib
            commonArgs
            cargoArtifacts
            gascityPackage
            beadsPackage
            ;
        };
    in
    {
      packages = forSystems (
        system:
        let
          context = mkContext system;
        in
        {
          default = context.craneLib.buildPackage (
            context.commonArgs
            // {
              inherit (context) cargoArtifacts;
              pname = "orchestrator";
              meta.mainProgram = "orchestrator";
            }
          );
        }
      );

      checks = forSystems (
        system:
        let
          context = mkContext system;
        in
        {
          default = context.craneLib.cargoTest (
            context.commonArgs
            // {
              inherit (context) cargoArtifacts;
            }
          );

          orchestrator-integration = context.craneLib.cargoTest (
            context.commonArgs
            // {
              inherit (context) cargoArtifacts;
              cargoExtraArgs = "--test integration_cascade -- --nocapture";
              nativeBuildInputs = [
                self.packages.${system}.default
                context.beadsPackage
                context.gascityPackage
                context.pkgs.bash
                context.pkgs.coreutils
                context.pkgs.codex
                context.pkgs.dolt
                context.pkgs.git
                context.pkgs.gnugrep
                context.pkgs.jq
                context.pkgs.lsof
                context.pkgs.python3
                context.pkgs.procps
                context.pkgs.tmux
                context.pkgs.util-linux
              ];
              ORCHESTRATOR_BIN = "${self.packages.${system}.default}/bin/orchestrator";
              ORCHESTRATOR_CODEX_PROVIDER_MODE = "shim";
              ORCHESTRATOR_RUN_GC_INTEGRATION = "1";
              ORCHESTRATOR_TEST_SCRIPT = "${./tests/scripts/orchestrator-isolated-gc-test.sh}";
              ORCHESTRATOR_TEST_CITY_TOML = "${./tests/fixtures/deterministic-city.toml}";
            }
          );
        }
      );

      devShells = forSystems (
        system:
        let
          context = mkContext system;
        in
        {
          default = context.pkgs.mkShell {
            packages = [
              context.toolchain
              context.pkgs.jq
            ];
          };
        }
      );
    };
}
