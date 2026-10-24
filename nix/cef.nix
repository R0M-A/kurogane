{
  lib,
  stdenv,
  fetchurl,
  cef-binary,
  linkFarm,
  symlinkJoin,
  writeTextFile,
}:

let
  version = "154.0.33";
  gitRevision = "a03e714";
  chromiumVersion = "154.0.8037.94";

  platform =
    {
      "x86_64-linux" = "linux64";
      "aarch64-linux" = "linuxarm64";
      "armv7l-linux" = "linuxarm";

      "x86_64-darwin" = "macosx64";
      "aarch64-darwin" = "macosarm64";

      "x86_64-windows" = "windows64";
      "aarch64-windows" = "windowsarm64";
      "i686-windows" = "windows32";
    }
    .${stdenv.hostPlatform.system} or (throw "Unsupported platform: ${stdenv.hostPlatform.system}");

  # Archive SHA-1 published in the CEF build index
  sha1 =
    {
      "x86_64-linux" = "9794ecf85ccd4dfcca42bfaac7a7666004f051e8";
      "aarch64-linux" = "87d20c0c25f08841db595c327125d270f5045865";
      "aarch64-darwin" = "bfa2358a5fba8d0a016118941d9cda0bf2d06f20";
    }
    .${stdenv.hostPlatform.system};

  cef =
    if stdenv.hostPlatform.isDarwin then
      stdenv.mkDerivation {
        pname = "cef-binary";
        inherit version;

        src = fetchurl {
          url = "https://cef-builds.spotifycdn.com/cef_binary_${version}+g${gitRevision}+chromium-${chromiumVersion}_${platform}_minimal.tar.bz2";

          hash =
            {
              "aarch64-darwin" = "sha256-beeJyUKulIWWsbDGDXJQPA4SbzaKvdT+5TDo6qN3aBQ=";
            }
            .${stdenv.hostPlatform.system};
        };

        dontConfigure = true;
        dontBuild = true;

        installPhase = ''
          cp -R . $out
        '';
      }
    else
      cef-binary.override {
        inherit version gitRevision chromiumVersion;

        srcHashes = {
          aarch64-linux = "sha256-zQ8IkqjCF7Z2DfafhZs70jFY/Kob7jXFWYpBxuCHiJk=";
          x86_64-linux = "sha256-zYnjZgVdRBKUL6JTNKn8mKW2n/Wwq/YgnJTi2eOABaI=";
        };
      };

  # CEF's license and credits which every bundle carries
  notices = linkFarm "cef-notices-${version}" (
    lib.genAttrs [ "LICENSE.txt" "CREDITS.html" ] (name: "${cef}/${name}")
  );

  archiveJson = writeTextFile {
    name = "cef-archive.json";
    destination = "/archive.json";
    text = builtins.toJSON {
      type = "minimal";
      name = "cef_binary_${version}+g${gitRevision}+chromium-${chromiumVersion}_${platform}_minimal.tar.bz2";
      inherit sha1;
    };
  };
in
symlinkJoin {
  pname = "cef-with-archive";
  inherit version;

  paths = [
    "${cef}/Release"
    archiveJson
    notices
  ]
  ++ lib.optional (!stdenv.hostPlatform.isDarwin) "${cef}/Resources";

  # Verifies the pinned archive against its declared SHA-1
  passthru.tests.cef-archive-sha1 = fetchurl {
    name = "cef-archive-sha1";
    url = "file://${cef.src}";
    inherit sha1;
  };
}
