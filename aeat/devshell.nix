{pkgs, ...}:
pkgs.mkShell {
  packages = [
    (pkgs.python3.withPackages (ps:
      with ps; [
        # Runtime deps (from pyproject.toml)
        httpx
        requests
        click

        # requests-pkcs12 — not in nixpkgs, install via pip
        # pip install requests-pkcs12

        # Dev/test deps
        pytest
        pytest-httpx

        # Tooling
        ruff
        pip
      ]))

    # System tools needed by submit.py
    pkgs.openssl

    # Nix tooling
    pkgs.alejandra
  ];

  env = {
    PYTHONDONTWRITEBYTECODE = "1";
  };

  shellHook = ''
    echo "aeat-autonomo devshell"
    echo "  python:  $(python3 --version)"
    echo "  openssl: $(openssl version)"
    echo ""
    echo "Run: pip install -e '.[dev]'  # for requests-pkcs12 + editable install"
  '';
}
