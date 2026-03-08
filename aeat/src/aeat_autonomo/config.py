"""Configuration and shared types."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class CertificateConfig:
    """FNMT certificate configuration for mTLS auth."""

    pfx_path: Path
    password: str

    def __post_init__(self) -> None:
        if not self.pfx_path.exists():
            raise FileNotFoundError(f"Certificate not found: {self.pfx_path}")


@dataclass(frozen=True)
class QuipuConfig:
    """Quipu API configuration."""

    api_key: str
    api_secret: str
    base_url: str = "https://getquipu.com/api"
