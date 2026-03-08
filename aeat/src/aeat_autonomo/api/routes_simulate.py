"""Simulate endpoint — preview a full fiscal year."""

from __future__ import annotations

from datetime import date
from decimal import Decimal

from fastapi import APIRouter, Depends, HTTPException, Query

from ..logging import get_logger
from ..modelo130 import Modelo130Data
from ..modelo303 import Modelo303Data
from ..quipu import QuipuClient
from .auth import require_api_key
from .config import ApiSettings
from .schemas import (
    AnnualSummary,
    QuipuTotalsResponse,
    SimulateQuarterResult,
    SimulateResponse,
    Summary130,
    Summary303,
)

log = get_logger(__name__)

router = APIRouter(prefix="/api/v1", tags=["simulate"], dependencies=[Depends(require_api_key)])


def _get_settings() -> ApiSettings:
    from . import _settings
    return _settings


@router.get("/simulate", response_model=SimulateResponse)
def simulate(
    year: int | None = Query(None, examples=[2026]),
    quarters: str = Query("1T,2T,3T,4T", examples=["1T,2T,3T,4T"]),
) -> SimulateResponse:
    """Simulate a full fiscal year using Quipu data.

    Generates both Modelo 303 and 130 for each quarter without submitting.
    """
    log.info("simulate_request", year=year, quarters=quarters)
    settings = _get_settings()
    if not settings.quipu.is_configured:
        raise HTTPException(status_code=400, detail="Quipu credentials not configured")

    if year is None:
        year = date.today().year

    quarter_list = [q.strip() for q in quarters.split(",")]
    valid = {"1T", "2T", "3T", "4T"}
    for q in quarter_list:
        if q not in valid:
            raise HTTPException(status_code=422, detail=f"Invalid quarter: {q}")

    max_q = max(int(q[0]) for q in quarter_list)
    quarters_to_fetch = [f"{i}T" for i in range(1, max_q + 1)]

    quarterly_data = {}
    with QuipuClient(settings.quipu) as client:
        for q in quarters_to_fetch:
            quarterly_data[q] = client.get_quarterly_totals(year, int(q[0]))

    accum_income = Decimal("0")
    accum_expenses = Decimal("0")
    accum_130_payments = Decimal("0")

    annual_vat_collected = Decimal("0")
    annual_vat_deductible = Decimal("0")
    annual_income = Decimal("0")
    annual_expenses = Decimal("0")

    results: list[SimulateQuarterResult] = []

    for q in quarters_to_fetch:
        totals = quarterly_data[q]
        accum_income += totals.total_income_gross
        accum_expenses += totals.total_expenses_gross

        m130 = Modelo130Data(
            nif=settings.declarant.nif,
            apellidos=settings.declarant.apellidos,
            nombre=settings.declarant.nombre,
            exercise=year,
            period=q,
            ingresos=accum_income,
            gastos=accum_expenses,
            pagos_anteriores=accum_130_payments,
            cuenta_iban=settings.iban,
        )

        if q not in quarter_list:
            if m130.resultado > 0:
                accum_130_payments += m130.resultado
            continue

        annual_vat_collected += totals.total_vat_collected
        annual_vat_deductible += totals.total_vat_deductible
        annual_income += totals.total_income_gross
        annual_expenses += totals.total_expenses_gross

        m303 = Modelo303Data(
            nif=settings.declarant.nif,
            nombre_razon=settings.declarant.nombre_completo,
            exercise=year,
            period=q,
            base_21=totals.total_income_gross,
            cuota_21=totals.total_vat_collected,
            base_deducible_interior=totals.total_expenses_gross,
            cuota_deducible_interior=totals.total_vat_deductible,
            cuenta_iban=settings.iban,
        )

        results.append(SimulateQuarterResult(
            quarter=q,
            quipu=QuipuTotalsResponse(
                year=year,
                quarter=q,
                total_income_gross=str(totals.total_income_gross),
                total_vat_collected=str(totals.total_vat_collected),
                total_expenses_gross=str(totals.total_expenses_gross),
                total_vat_deductible=str(totals.total_vat_deductible),
                net_vat=str(totals.net_vat),
                net_income=str(totals.net_income),
            ),
            modelo_303=Summary303(
                iva_devengado=str(m303.total_cuota_devengada),
                iva_deducible=str(m303.total_a_deducir),
                resultado=str(m303.resultado),
                resultado_liquidacion=str(m303.resultado_liquidacion),
                tipo_declaracion=m303.tipo_declaracion,
            ),
            modelo_130=Summary130(
                ingresos=str(m130.ingresos),
                gastos=str(m130.gastos),
                rendimiento_neto=str(m130.rendimiento_neto),
                pago_20_pct=str(m130.pago_20_pct),
                pagos_anteriores=str(m130.pagos_anteriores),
                resultado=str(m130.resultado),
                tipo_declaracion=m130.tipo_declaracion,
            ),
        ))

        if m130.resultado > 0:
            accum_130_payments += m130.resultado

    log.info("simulate_complete", year=year, quarters_count=len(results))

    return SimulateResponse(
        year=year,
        quarters=results,
        annual_summary=AnnualSummary(
            total_income=str(annual_income),
            total_expenses=str(annual_expenses),
            net_income=str(annual_income - annual_expenses),
            total_vat_collected=str(annual_vat_collected),
            total_vat_deducted=str(annual_vat_deductible),
            net_vat_paid=str(annual_vat_collected - annual_vat_deductible),
            irpf_advance_paid=str(accum_130_payments),
        ),
    )
