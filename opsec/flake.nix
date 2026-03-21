{
  description = "OPSEC - OSINT reconnaissance tools (RIPE NCC, BGPView, Shodan)";

  inputs = {
    blueprint.url = "github:numtide/blueprint";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs:
    inputs.blueprint {
      inherit inputs;
      prefix = "nix/";
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
    };
}
