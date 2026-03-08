"""API configuration — Pydantic BaseSettings with file + env override."""

from __future__ import annotations

from pathlib import Path

from pydantic import model_validator
from pydantic_settings import BaseSettings

from ..config import AeatConfig, CertificateConfig, DeclarantConfig, QuipuConfig


class ApiSettings(BaseSettings):
    """Settings loaded from environment variables, with optional file fallback.

    Priority (highest wins): env vars > config file values > defaults.
    """

    # API server
    host: str = "0.0.0.0"
    port: int = 8000
    api_key: str = ""

    # Nested config (populated from file, overridable via env)
    declarant: DeclarantConfig = DeclarantConfig()
    certificate: CertificateConfig = CertificateConfig()
    quipu: QuipuConfig = QuipuConfig()
    iban: str = ""
    testing: bool = True

    # Config file path (used to load defaults before env override)
    config: str = "aeat-config.json"

    # Flat env var aliases for certificate/quipu/declarant fields
    cert_path: str = ""
    cert_password: str = ""
    quipu_key: str = ""
    quipu_secret: str = ""
    declarant_nif: str = ""
    declarant_apellidos: str = ""
    declarant_nombre: str = ""

    model_config = {"env_prefix": "AEAT_"}

    @model_validator(mode="after")
    def _apply_flat_env_overrides(self) -> ApiSettings:
        """Merge flat env vars (AEAT_CERT_PATH, etc.) into nested models."""
        if self.cert_path or self.cert_password:
            merged = self.certificate.model_dump()
            if self.cert_path:
                merged["pfx_path"] = self.cert_path
            if self.cert_password:
                merged["password"] = self.cert_password
            object.__setattr__(self, "certificate", CertificateConfig(**merged))

        if self.quipu_key or self.quipu_secret:
            merged = self.quipu.model_dump()
            if self.quipu_key:
                merged["api_key"] = self.quipu_key
            if self.quipu_secret:
                merged["api_secret"] = self.quipu_secret
            object.__setattr__(self, "quipu", QuipuConfig(**merged))

        if self.declarant_nif or self.declarant_apellidos or self.declarant_nombre:
            merged = self.declarant.model_dump()
            if self.declarant_nif:
                merged["nif"] = self.declarant_nif
            if self.declarant_apellidos:
                merged["apellidos"] = self.declarant_apellidos
            if self.declarant_nombre:
                merged["nombre"] = self.declarant_nombre
            object.__setattr__(self, "declarant", DeclarantConfig(**merged))

        return self


def load_settings() -> ApiSettings:
    """Load settings: read config file first, then let env vars override."""
    # First, figure out config file path from env or default
    preliminary = ApiSettings()
    config_path = Path(preliminary.config)

    file_values: dict = {}
    if config_path.exists():
        aeat_config = AeatConfig.from_file(config_path)
        file_values = {
            "declarant": aeat_config.declarant,
            "certificate": aeat_config.certificate,
            "quipu": aeat_config.quipu,
            "iban": aeat_config.iban,
            "testing": aeat_config.testing,
        }

    # Rebuild settings: file values serve as defaults, env vars override via model_validator
    return ApiSettings(**file_values)
