{pkgs, ...}:
pkgs.mkShell {
  packages = [
    (pkgs.python3.withPackages (ps:
      with ps; [
        requests
        shodan
        scapy
      ]))
    pkgs.netcat-gnu
  ];
}
