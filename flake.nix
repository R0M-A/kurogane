{
  description = "Kurogane";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
    }:
    let
      cefVersion = "150.0.10";
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      craneLib = (crane.mkLib pkgs);

      runtimeDeps = with pkgs; [
        openssl
        dbus
        at-spi2-core
        glib
        libGL
        libxkbcommon
        wayland
        libX11
        libXcomposite
        libXcursor
        libXdamage
        libXext
        libXfixes
        libXi
        libXrandr
        libXrender
        libXScrnSaver
        libXtst
        libxcb
        gtk3
        nss
        nspr
        pango
        cairo
        alsa-lib
        at-spi2-atk
        atk
        cups
        expat
        fontconfig
        gdk-pixbuf
        libva
        libgbm
        libvdpau
        systemd
      ];

      buildDeps = with pkgs; [
        rustc
        cargo
        rsync
        pkg-config
        cmake
        ninja
        gcc
        cef
      ];

      cefIntermediate = pkgs.cef-binary.override {
        version = "150.0.10";
        gitRevision = "8042e43";
        chromiumVersion = "150.0.7871.101";
        srcHashes = {
          aarch64-linux = "";
          x86_64-linux = "sha256-bB1Ike84huPM9l0JKI2DBOP343JKR8kyk+K9Y+dlKOQ=";
        };
      };

      cef = cefIntermediate.overrideAttrs (old: {
        postInstall = (old.postInstall or "") + ''
          cat > "$out/archive.json" <<EOF
          {
            "type": "minimal",
            "name": "cef_binary_${cefVersion}",
            "sha1": "0000000000000000000000000000000000000000"
          }
          EOF

          ln -sf $out/Release/* $out/
          ln -sf $out/Resources/* $out/
        '';
      });


      kurogane = craneLib.buildPackage {
        src = ./.;
        cargoArtifacts = craneLib.vendorCargoDeps { src = ./.; };

        pname = "kurogane";
        version = "0.4.0";

        cargoExtraArgs = "-p kurogane-cli";

        buildInputs = runtimeDeps;
        nativeBuildInputs = buildDeps ++ [
          pkgs.autoPatchelfHook
          pkgs.makeWrapper
        ];

        postInstall = ''
          wrapProgram $out/bin/kurogane \
            --set KUROGANE_CEF_VERSION ${cefVersion} \
            --set CEF_PATH ${cef} \
            --prefix PATH : ${
              pkgs.lib.makeBinPath (
                buildDeps
                ++ [
                  pkgs.rustc
                  pkgs.cargo
                  pkgs.cmake
                  pkgs.ninja
                  pkgs.pkg-config
                ]
              )
            } \
            --prefix LD_LIBRARY_PATH : ${
              pkgs.lib.makeLibraryPath (runtimeDeps ++ [ pkgs.stdenv.cc.cc ])
            }:${cef} \
            --prefix PKG_CONFIG_PATH : ${pkgs.lib.makeSearchPath "lib/pkgconfig" runtimeDeps}
        '';
      };
    in
    {

      apps.${system}.default = {
        type = "app";
        program = "${kurogane}/bin/kurogane";
      };

      devShells.${system}.default = pkgs.mkShell {
        buildInputs =
          runtimeDeps
          ++ buildDeps
          ++ [
            pkgs.autoPatchelfHook
            kurogane
          ];
      };
    };
}
