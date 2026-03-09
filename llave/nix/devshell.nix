{
  inputs,
  pkgs,
  system,
  ...
}:
let
  fenixPkgs = inputs.fenix.packages.${system};
  toolchain = fenixPkgs.stable.withComponents [
    "cargo"
    "clippy"
    "rust-src"
    "rustc"
    "rustfmt"
  ];
in
pkgs.mkShell {
  buildInputs =
    [
      toolchain
      fenixPkgs.rust-analyzer

      pkgs.openssl
      pkgs.pkg-config
      pkgs.dbus

      # Flutter + desktop/mobile development
      pkgs.flutter
      pkgs.dart

      # Flutter Linux desktop dependencies
      pkgs.gtk3
      # glib/gio private deps — their .pc files must be on PKG_CONFIG_PATH
      # because Flutter's cmake probes them transitively via Requires.private.
      pkgs.glib.dev
      pkgs.libsysprof-capture
      pkgs.pcre2
      pkgs.util-linux.dev    # mount.pc, required by gio-2.0
      pkgs.libselinux        # libselinux.pc, required by gio-2.0
      pkgs.libsepol          # libsepol.pc, required by libselinux
      pkgs.fribidi           # fribidi.pc, required by pango
      pkgs.libthai           # libthai.pc, required by pango
      pkgs.libdatrie         # libdatrie.pc, required by libthai
      pkgs.libxdmcp           # xdmcp.pc, required by xcb
      pkgs.pcre              # libpcre.pc (v1), required by some transitive deps
      pkgs.libepoxy          # epoxy.pc, required by gtk3
      pkgs.libdeflate        # libdeflate.pc, required by libtiff-4
      pkgs.libgcrypt         # libgcrypt.pc, required by libsecret-1
      pkgs.libsecret  # flutter_secure_storage on Linux
      pkgs.jsoncpp    # flutter_secure_storage on Linux
      pkgs.cmake
      pkgs.ninja
      pkgs.clang

      # APK reverse engineering tools
      pkgs.apktool
      pkgs.jadx
    ]
    ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
      pkgs.libiconv
      pkgs.darwin.apple_sdk.frameworks.Security
      pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
    ];

  RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";

  shellHook = ''
    # Ensure flutter_rust_bridge_codegen is available
    if ! command -v flutter_rust_bridge_codegen &>/dev/null; then
      echo "Installing flutter_rust_bridge_codegen..."
      cargo install flutter_rust_bridge_codegen@2.11.1
    fi
  '';
}
