"""Configuration models — shared across CLI and API.

The JSON config file structure maps directly to AeatConfig:

    {
        "declarant": {"nif": "...", "apellidos": "...", "nombre": "..."},
        "certificate": {"pfx_path": "./cert.p12", "password": "..."},
        "quipu": {"api_key": "...", "api_secret": "..."},
        "iban": "ES...",
        "testing": true
    }
"""

from __future__ import annotations

from pathlib import Path

from pydantic import BaseModel, model_validator


class DeclarantConfig(BaseModel, frozen=True):
    """Declarant identity (NIF + name)."""

    nif: str = ""
    apellidos: str = ""
    nombre: str = ""

    @property
    def nombre_completo(self) -> str:
        return f"{self.apellidos} {self.nombre}".strip()


class CertificateConfig(BaseModel, frozen=True):
    """FNMT certificate configuration for mTLS auth."""

    pfx_path: Path = Path("")
    password: str = ""

    @property
    def is_configured(self) -> bool:
        return bool(self.pfx_path.name and self.password)

    @model_validator(mode="after")
    def _check_file_exists(self) -> CertificateConfig:
        if self.is_configured and not self.pfx_path.exists():
            raise FileNotFoundError(f"Certificate not found: {self.pfx_path}")
        return self


class QuipuConfig(BaseModel, frozen=True):
    """Quipu API configuration."""

    api_key: str = ""
    api_secret: str = ""
    base_url: str = "https://getquipu.com/api"

    @property
    def is_configured(self) -> bool:
        return bool(self.api_key and self.api_secret)


class AeatConfig(BaseModel, frozen=True):
    """Top-level configuration — mirrors the JSON config file structure."""

    declarant: DeclarantConfig = DeclarantConfig()
    certificate: CertificateConfig = CertificateConfig()
    quipu: QuipuConfig = QuipuConfig()
    iban: str = ""
    testing: bool = True

    @classmethod
    def from_file(cls, path: Path) -> AeatConfig:
        """Load configuration from a JSON file."""
        return cls.model_validate_json(path.read_text())
