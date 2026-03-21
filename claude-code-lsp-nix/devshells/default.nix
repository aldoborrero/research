{pkgs, ...}:
pkgs.mkShell {
  packages = [
    pkgs.nixd
  ];
}
