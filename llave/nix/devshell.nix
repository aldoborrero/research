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
      cargo install flutter_rust_bridge_codegen@2.9.0
    fi
  '';
}
