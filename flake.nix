{
  description = "Kurogane";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    kurogane-nixpkgs.url = "github:0x48piraj/nixpkgs?ref=cef-binary-optional-angle-libs";

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
        kurogane-nixpkgs = import inputs.kurogane-nixpkgs { inherit system; };
        kurogane = pkgs.callPackage ./nix/package.nix { inherit kurogane-nixpkgs; };

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          version = "dev";
          cargoExtraArgs = "-p kurogane-cli";

          # rustc and cargo conflict with craneLib's own rustc and cargo
          nativeBuildInputs = pkgs.lib.subtractLists [ pkgs.rustc pkgs.cargo ] kurogane.nativeBuildInputs;
          inherit (kurogane)
            pname
            buildInputs
            strictDeps
            USER
            CEF_PATH
            meta
            ;
        };

        kurogane-dev = craneLib.buildPackage (
          commonArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
            inherit (kurogane) postInstall;
          }
        );
      in
      {
        packages = {
          default = kurogane-dev;
          inherit kurogane-dev kurogane;
          inherit (kurogane) cef-binary;
        };

        apps.default = {
          type = "app";
          program = "${kurogane}/bin/kurogane";
          inherit (kurogane) meta;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ kurogane-dev ];
          packages = with pkgs; [
            kurogane-dev
            rustc
            cargo
            clippy
            rustfmt
          ];

          inherit (kurogane) CEF_PATH;
        };

        checks = pkgs.lib.mergeAttrsList [
          {
            inherit kurogane-dev;
            bundle-fixture = import ./nix/bundleFixture.nix {
              inherit pkgs craneLib kurogane;
              src = ./.;
            };
          }
          kurogane.cef-binary.tests
        ];
      }
    );
}
