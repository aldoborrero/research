"""Submit/validate endpoints — send declarations to AEAT."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException

from ..logging import get_logger
from .auth import require_api_key
from .schemas import SubmitRequest, SubmitResponse

log = get_logger(__name__)

router = APIRouter(prefix="/api/v1", tags=["submit"], dependencies=[Depends(require_api_key)])


def _get_client():
    from . import get_submit_client
    client = get_submit_client()
    if client is None:
        raise HTTPException(status_code=400, detail="Certificate not configured")
    return client


@router.post("/submit/303", response_model=SubmitResponse)
def submit_303(req: SubmitRequest) -> SubmitResponse:
    """Submit Modelo 303 to AEAT via Presentacion Directa."""
    client = _get_client()

    log.info("submit_303_request", year=req.year, quarter=req.quarter, dry_run=req.dry_run)

    if req.dry_run:
        result = client.validate("303", str(req.year), req.quarter, req.boe_content)
    else:
        result = client.submit(
            "303", str(req.year), req.quarter, req.boe_content, nrc=req.nrc,
        )

    log.info(
        "submit_303_response",
        year=req.year,
        quarter=req.quarter,
        success=result.success,
        dry_run=req.dry_run,
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
    client = _get_client()

    log.info("submit_130_request", year=req.year, quarter=req.quarter, dry_run=req.dry_run)

    if req.dry_run:
        result = client.validate("130", str(req.year), req.quarter, req.boe_content)
    else:
        result = client.submit(
            "130", str(req.year), req.quarter, req.boe_content, nrc=req.nrc,
        )

    log.info(
        "submit_130_response",
        year=req.year,
        quarter=req.quarter,
        success=result.success,
        dry_run=req.dry_run,
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
