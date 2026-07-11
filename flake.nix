{
  description = "Kurogane";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      crane,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = (crane.mkLib pkgs);

        runtimeDeps = with pkgs; [
          openssl
          dbus
          at-spi2-core
          glib
          libGL
          libxkbcommon
          wayland
          xorg.libX11
          xorg.libXcomposite
          xorg.libXcursor
          xorg.libXdamage
          xorg.libXext
          xorg.libXfixes
          xorg.libXi
          xorg.libXrandr
          xorg.libXrender
          xorg.libXScrnSaver
          xorg.libXtst
          xorg.libxcb
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

        buildDeps =
          with pkgs;
          [
            rustc
            cargo
            rsync
            pkg-config
            cmake
            ninja
            gcc
          ]
          ++ [ cef ];

        cef = pkgs.stdenvNoCC.mkDerivation {
          pname = "cef-binary";
          version = "150.0.10";

          src = pkgs.fetchurl {
            url = "https://cef-builds.spotifycdn.com/cef_binary_150.0.10%2Bg8042e43%2Bchromium-150.0.7871.101_linux64.tar.bz2";
            sha256 = "sha256-FpDfe8eLVtgIyBdBmLzIsuEFT+Mjk5OrdVaDRnE4V64=";
          };

          dontBuild = true;

          nativeBuildInputs = [ pkgs.lbzip2 ];

          installPhase = ''
            dest=$out/${cef.version}/cef_linux_x86_64
            mkdir -p $dest
            tar --use-compress-program=lbzip2 -xf $src -C $dest --strip-components=1

            # Move contents of Release and Resources to the root of the CEF installation
            # This matches the "flattened" layout cef-dll-sys expects
            cp -r $dest/Release/* $dest/
            cp -r $dest/Resources/* $dest/
            rm -rf $dest/Release $dest/Resources $dest/Debug

            # Write archive.json (cef-dll-sys build script validates version via this file)
            cat > $out/${cef.version}/cef_linux_x86_64/archive.json << 'EOF'
            {
              "type": "minimal",
              "name": "cef_binary_${cef.version}",
              "sha1": "0000000000000000000000000000000000000000"
            }
            EOF
          '';
        };

        kurogane =
          let
            src = ./.;

            cargoArtifacts = craneLib.vendorCargoDeps {
              inherit src;
            };

          in
          craneLib.buildPackage {
            inherit src cargoArtifacts;

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
                --set KUROGANE_CEF_VERSION ${cef.version} \
                --set CEF_PATH ${cef}/${cef.version}/cef_linux_x86_64 \
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
                }:${cef}/${cef.version}/cef_linux_x86_64 \
                --prefix PKG_CONFIG_PATH : ${pkgs.lib.makeSearchPath "lib/pkgconfig" runtimeDeps}
            '';
          };
      in
      {
        packages = {
          inherit kurogane;
          default = kurogane;
        };

        apps.default = {
          type = "app";
          program = "${kurogane}/bin/kurogane";
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = buildDeps ++ [ pkgs.autoPatchelfHook ];
          buildInputs = runtimeDeps ++ [ kurogane ];

        };
      }
    );
}
