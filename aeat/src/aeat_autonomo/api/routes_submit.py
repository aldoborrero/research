"""Submit/validate endpoints — send declarations to AEAT."""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException

from ..config import CertificateConfig
from ..submit import PresentacionDirectaClient
from .auth import require_api_key
from .config import ApiSettings
from .schemas import SubmitRequest, SubmitResponse

router = APIRouter(prefix="/api/v1", tags=["submit"], dependencies=[Depends(require_api_key)])


def _get_settings() -> ApiSettings:
    from . import _settings
    return _settings


def _make_client(settings: ApiSettings) -> PresentacionDirectaClient:
    """Create a PresentacionDirectaClient from API settings."""
    if not settings.cert_path or not settings.cert_password:
        raise HTTPException(status_code=400, detail="Certificate not configured")

    cert_cfg = CertificateConfig(
        pfx_path=Path(settings.cert_path),
        password=settings.cert_password,
    )
    nombre = f"{settings.declarant_apellidos} {settings.declarant_nombre}".strip()

    return PresentacionDirectaClient(
        cert_cfg,
        nif_presentador=settings.declarant_nif,
        nombre_presentador=nombre,
        testing=settings.testing,
    )


@router.post("/submit/303", response_model=SubmitResponse)
def submit_303(req: SubmitRequest) -> SubmitResponse:
    """Submit Modelo 303 to AEAT via Presentacion Directa."""
    settings = _get_settings()
    with _make_client(settings) as client:
        if req.dry_run:
            result = client.validate("303", str(req.year), req.quarter, req.boe_content)
        else:
            result = client.submit(
                "303", str(req.year), req.quarter, req.boe_content, nrc=req.nrc,
            )

    return SubmitResponse(
        success=result.success,
        csv=result.csv,
        justificante=result.justificante,
        fecha=result.fecha,
        hora=result.hora,
        pdf_url=result.pdf_url,
        pdf_base64=result.pdf_base64,
        warnings=result.warnings,
        errors=result.errors,
    )


@router.post("/submit/130", response_model=SubmitResponse)
def submit_130(req: SubmitRequest) -> SubmitResponse:
    """Submit Modelo 130 to AEAT via Presentacion Directa."""
    settings = _get_settings()
    with _make_client(settings) as client:
        if req.dry_run:
            result = client.validate("130", str(req.year), req.quarter, req.boe_content)
        else:
            result = client.submit(
                "130", str(req.year), req.quarter, req.boe_content, nrc=req.nrc,
            )

    return SubmitResponse(
        success=result.success,
        csv=result.csv,
        justificante=result.justificante,
        fecha=result.fecha,
        hora=result.hora,
        pdf_url=result.pdf_url,
        pdf_base64=result.pdf_base64,
        warnings=result.warnings,
        errors=result.errors,
    )
