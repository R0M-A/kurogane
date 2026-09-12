{
  lib,
  stdenv,
  rustPlatform,
  rustc,
  cargo,
  openssl,
  dbus,
  at-spi2-core,
  glib,
  libGL,
  libxkbcommon,
  wayland,
  libX11,
  libXcomposite,
  libXcursor,
  libXdamage,
  libXext,
  libXfixes,
  libXi,
  libXrandr,
  libXrender,
  libXScrnSaver,
  libXtst,
  libxcb,
  gtk3,
  nss,
  nspr,
  pango,
  cairo,
  alsa-lib,
  at-spi2-atk,
  atk,
  cups,
  expat,
  fontconfig,
  gdk-pixbuf,
  libva,
  libgbm,
  libvdpau,
  systemd,
  pkg-config,
  cmake,
  cef-binary,
  fetchFromGitHub,
  versionCheckHook,
  makeWrapper,
  callPackage,
}:
let
  src = fetchFromGitHub {
    owner = "0x48piraj";
    repo = "kurogane";
    tag = "0.0.5-alpha.2";
    hash = "sha256-4ihCH6mzJabZ3V9IDomKUFyA1POa6e5ipD3GT/I/6Qw=";
  };

  cef = callPackage ./cef.nix { };

  buildInputs =
            [
              openssl
              nss
              nspr
              pango
              cairo
              glib
              expat
              fontconfig
            ]
            ++ lib.optionals stdenv.hostPlatform.isLinux [
              dbus
              at-spi2-core
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
              alsa-lib
              at-spi2-atk
              atk
              cups
              gdk-pixbuf
              libva
              libgbm
              libvdpau
              systemd
            ];

  nativeBuildInputs = [
    rustc
    cargo
    pkg-config
    makeWrapper
    cmake
  ];
in
rustPlatform.buildRustPackage {
  pname = "kurogane";
  version = src.tag;
  cargoHash = "sha256-Wy8npQlgZj4C6lQ6MIIgQVSbcUoDKM98WN8QUgnAJnM=";

  inherit
    src
    cef
    buildInputs
    nativeBuildInputs
    ;

  strictDeps = true;

  #     cargoExtraArgs = "-p kurogane-cli";

  USER = "Kurogane Tests"; # Fallback when git.user and git.email aren't set
  CEF_PATH = cef;

  postInstall = ''
    wrapProgram $out/bin/kurogane \
      --set CEF_PATH ${cef} \
      --prefix PATH : ${lib.makeBinPath nativeBuildInputs} \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath (buildInputs ++ [ stdenv.cc.cc ])}:${cef} \
      --prefix PKG_CONFIG_PATH : ${lib.makeSearchPath "lib/pkgconfig" buildInputs}
  '';

  #   checkFlags = [
  #     "--skip=cache::tests::first_acquire_clones_and_second_reuses_without_network"
  #     "--skip=cache::tests::corrupt_entries_are_removed_and_recloned"
  #   ];
  #   doInstallCheck = true;
  #   nativeInstallCheckInputs = [ versionCheckHook ];

  meta = {
    description = "Composable Chromium runtime for Rust";
    longDescription = ''
      Kurogane is a Rust-native runtime built on [Chromium Embedded Framework (CEF)](https://en.wikipedia.org/wiki/Chromium_Embedded_Framework), bringing Chromium to desktop applications while giving you control over windowing, event loops and lifecycle when you need it.
    '';
    homepage = "https://github.com/0x48piraj/kurogane";
    changelog = "https://github.com/0x48piraj/kurogane/releases";
    # Mirrors the allow-list in cargo-deny
    license = with lib.licenses; [
      asl20
      llvm-exception
      bsd2
      bsd3
      isc
      mit
      zlib
      mpl20
      cc0
      ofl
      unicode-30
      bzip2
      {
        free = true;
        shortName = "LicenseRef-UFL-1.0";
      }
      {
        free = true;
        shortName = "CDLA-Permissive-2.0";
      }
    ];
    sourceProvenance = with lib.sourceTypes; [ fromSource ] ++ cef-binary.meta.sourceProvenance; # cef is binaryNativeCode
    maintainers = with lib.maintainers; [
      _0x48piraj
      R0M-A
    ]; # TODO: get on maintainers list
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "kurogane";
  };
}
