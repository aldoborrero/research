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
class AEATEndpoints:
    """AEAT endpoint URLs."""

    # Servicios Comunes (for 303, 130, 390)
    servicios_comunes: str
    # TGVI Online (for 347, 349)
    tgvi_online: str

    @classmethod
    def production(cls) -> AEATEndpoints:
        return cls(
            servicios_comunes="https://www1.agenciatributaria.gob.es/wlpl/SSII-FACT/ws/fe/SiiFactFEV1SOAP",
            tgvi_online="https://www1.agenciatributaria.gob.es/wlpl/inwinvoc/es.aeat.dit.adi.eama.jdit.ws.DRServicioDeclaracionREST",
        )

    @classmethod
    def testing(cls) -> AEATEndpoints:
        return cls(
            servicios_comunes="https://prewww1.aeat.es/wlpl/SSII-FACT/ws/fe/SiiFactFEV1SOAP",
            tgvi_online="https://prewww2.aeat.es/wlpl/inwinvoc/es.aeat.dit.adi.eama.jdit.ws.DRServicioDeclaracionREST",
        )


@dataclass(frozen=True)
class QuipuConfig:
    """Quipu API configuration."""

    api_key: str
    api_secret: str
    base_url: str = "https://getquipu.com/api"
