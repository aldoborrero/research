{
  inputs,
  pkgs,
  system,
  ...
}:
let
  craneLib = inputs.crane.mkLib pkgs;

  src = craneLib.cleanCargoSource (craneLib.path ../../..);

  commonArgs = {
    inherit src;
    strictDeps = true;

    buildInputs =
      [ pkgs.openssl ]
      ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
        pkgs.libiconv
        pkgs.darwin.apple_sdk.frameworks.Security
        pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
      ];

    nativeBuildInputs = [
      pkgs.pkg-config
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;
    cargoExtraArgs = "--package clave-web";

    meta = {
      description = "Web interface for Cl@ve AEAT authentication";
      mainProgram = "clave-web";
    };
  }
)
