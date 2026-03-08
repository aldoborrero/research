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

      # Flutter + mobile development
      pkgs.flutter
      pkgs.dart

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
}
