{
  description = "Kurogane";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { self, nixpkgs, ... }@inputs:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
    in
    inputs.flake-utils.lib.eachSystem supportedSystems (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = inputs.crane.mkLib pkgs;

        kuroganePackage = pkgs.callPackage ./nix/package.nix { };
        cef = kuroganePackage.cef;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          inherit (kuroganePackage)
            buildInputs
            strictDeps
            meta
            pname
            ;
          # Can't inherit nativeBuildInputs because of cargo/rustc collision
          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
            cmake
          ];

          cargoExtraArgs = "-p kurogane-cli";

          USER = "Kurogane Tests"; # Fallback when git.user and git.email aren't set
          CEF_PATH = cef; # Don't download cef
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        kurogane = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "kurogane";
            version = "dev";

            postInstall = ''
              wrapProgram $out/bin/kurogane \
                --set CEF_PATH ${cef} \
                --prefix PATH : ${pkgs.lib.makeBinPath commonArgs.nativeBuildInputs} \
                --prefix LD_LIBRARY_PATH : ${
                  pkgs.lib.makeLibraryPath (commonArgs.buildInputs ++ [ pkgs.stdenv.cc.cc ])
                }:${cef} \
                --prefix PKG_CONFIG_PATH : ${pkgs.lib.makeSearchPath "lib/pkgconfig" commonArgs.buildInputs}
            '';
          }
        );
      in
      {
        packages = {
          default = kurogane;
          inherit kurogane kuroganePackage; # kuroganePackage is debug-only
        };

        apps.default = {
          type = "app";
          program = "${kurogane}/bin/kurogane";
          meta = kurogane.meta;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ kurogane ];
          packages = with pkgs; [
            kurogane
            rustc
            cargo
            clippy
            rustfmt
          ];

          # Reuse the Nix store CEF instead of letting cef-dll-sys re-download
          CEF_PATH = cef;
        };

        checks = pkgs.lib.mergeAttrsList [
          (import ./nix/testTemplates.nix { inherit pkgs kurogane; })
        ];
      }
    );
}
