"""Quipu endpoints — fetch accounting data."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, Query

from ..config import QuipuConfig
from ..logging import get_logger
from ..quipu import QuipuClient
from .auth import require_api_key
from .config import ApiSettings
from .schemas import Quarter, QuipuTotalsResponse

log = get_logger(__name__)

router = APIRouter(prefix="/api/v1/quipu", tags=["quipu"], dependencies=[Depends(require_api_key)])


def _get_settings() -> ApiSettings:
    from . import _settings
    return _settings


@router.get("/totals", response_model=QuipuTotalsResponse)
def quipu_totals(
    year: int = Query(..., examples=[2026]),
    quarter: Quarter = Query(..., examples=["1T"]),
) -> QuipuTotalsResponse:
    """Fetch quarterly totals from Quipu."""
    log.info("quipu_totals_request", year=year, quarter=quarter)
    settings = _get_settings()
    if not settings.quipu_key:
        raise HTTPException(status_code=400, detail="Quipu credentials not configured")

    quipu_cfg = QuipuConfig(api_key=settings.quipu_key, api_secret=settings.quipu_secret)
    q_num = int(quarter[0])

    with QuipuClient(quipu_cfg) as client:
        totals = client.get_quarterly_totals(year, q_num)

    return QuipuTotalsResponse(
        year=year,
        quarter=quarter,
        total_income_gross=str(totals.total_income_gross),
        total_vat_collected=str(totals.total_vat_collected),
        total_expenses_gross=str(totals.total_expenses_gross),
        total_vat_deductible=str(totals.total_vat_deductible),
        net_vat=str(totals.net_vat),
        net_income=str(totals.net_income),
    )
