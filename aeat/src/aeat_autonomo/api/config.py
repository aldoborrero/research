"""API configuration — Pydantic BaseSettings with file + env override."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from pydantic_settings import BaseSettings


class ApiSettings(BaseSettings):
    """Settings loaded from environment variables, with optional file fallback.

    Priority (highest wins): env vars > config file values > defaults.
    """

    # API server
    host: str = "0.0.0.0"
    port: int = 8000
    api_key: str = ""

    # AEAT certificate
    cert_path: str = ""
    cert_password: str = ""

    # Quipu
    quipu_key: str = ""
    quipu_secret: str = ""

    # Declarant
    declarant_nif: str = ""
    declarant_apellidos: str = ""
    declarant_nombre: str = ""
    iban: str = ""

    # Flags
    testing: bool = True

    # Config file path (used to load defaults before env override)
    config: str = "aeat-config.json"

    model_config = {"env_prefix": "AEAT_"}


def load_settings() -> ApiSettings:
    """Load settings: read config file first, then let env vars override."""
    # First, figure out config file path from env or default
    preliminary = ApiSettings()
    config_path = Path(preliminary.config)

    file_values: dict[str, Any] = {}
    if config_path.exists():
        raw = json.loads(config_path.read_text())
        declarant = raw.get("declarant", {})
        cert = raw.get("certificate", {})
        quipu = raw.get("quipu", {})

        file_values = {
            "declarant_nif": declarant.get("nif", ""),
            "declarant_apellidos": declarant.get("apellidos", ""),
            "declarant_nombre": declarant.get("nombre", ""),
            "cert_path": cert.get("pfx_path", ""),
            "cert_password": cert.get("password", ""),
            "quipu_key": quipu.get("api_key", ""),
            "quipu_secret": quipu.get("api_secret", ""),
            "iban": raw.get("iban", ""),
            "testing": raw.get("testing", True),
        }

    # Rebuild settings: file values serve as defaults, env vars override
    return ApiSettings(**{
        k: v for k, v in file_values.items() if v
    })
