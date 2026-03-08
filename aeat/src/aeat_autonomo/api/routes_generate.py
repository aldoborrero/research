"""Generate endpoints — create BOE files for Modelo 303 and 130."""

from __future__ import annotations

from decimal import Decimal

from fastapi import APIRouter, Depends, HTTPException

from ..config import QuipuConfig
from ..logging import get_logger
from ..modelo130 import Modelo130Data, generate_130_boe
from ..modelo303 import Modelo303Data, generate_303_boe
from ..quipu import QuipuClient
from .auth import require_api_key
from .config import ApiSettings

log = get_logger(__name__)
from .schemas import (
    Generate130Request,
    Generate130Response,
    Generate303Request,
    Generate303Response,
    Summary130,
    Summary303,
)

router = APIRouter(prefix="/api/v1/generate", tags=["generate"], dependencies=[Depends(require_api_key)])


def _get_settings() -> ApiSettings:
    from . import _settings
    return _settings


@router.post("/303", response_model=Generate303Response)
def generate_303(req: Generate303Request) -> Generate303Response:
    """Generate a Modelo 303 BOE file (quarterly VAT)."""
    log.info("generate_303_request", year=req.year, quarter=req.quarter, from_quipu=req.from_quipu)
    settings = _get_settings()
    nombre_completo = f"{settings.declarant_apellidos} {settings.declarant_nombre}".strip()

    if req.from_quipu:
        if not settings.quipu_key:
            raise HTTPException(status_code=400, detail="Quipu credentials not configured")
        quipu_cfg = QuipuConfig(api_key=settings.quipu_key, api_secret=settings.quipu_secret)
        q_num = int(req.quarter[0])
        with QuipuClient(quipu_cfg) as client:
            totals = client.get_quarterly_totals(req.year, q_num)
        data = Modelo303Data(
            nif=settings.declarant_nif,
            nombre_razon=nombre_completo,
            exercise=req.year,
            period=req.quarter,
            base_21=totals.total_income_gross,
            cuota_21=totals.total_vat_collected,
            base_deducible_interior=totals.total_expenses_gross,
            cuota_deducible_interior=totals.total_vat_deductible,
            cuenta_iban=settings.iban,
        )
    else:
        data = Modelo303Data(
            nif=settings.declarant_nif,
            nombre_razon=nombre_completo,
            exercise=req.year,
            period=req.quarter,
            base_21=Decimal(req.base_21),
            cuota_21=Decimal(req.cuota_21),
            base_10=Decimal(req.base_10),
            cuota_10=Decimal(req.cuota_10),
            base_4=Decimal(req.base_4),
            cuota_4=Decimal(req.cuota_4),
            base_deducible_interior=Decimal(req.base_deducible_interior),
            cuota_deducible_interior=Decimal(req.cuota_deducible_interior),
            cuenta_iban=settings.iban,
        )

    boe = generate_303_boe(data)

    log.info(
        "generate_303_complete",
        year=req.year,
        quarter=req.quarter,
        resultado=str(data.resultado),
        tipo=data.tipo_declaracion,
    )

    return Generate303Response(
        boe_content=boe,
        summary=Summary303(
            iva_devengado=str(data.total_cuota_devengada),
            iva_deducible=str(data.total_a_deducir),
            resultado=str(data.resultado),
            resultado_liquidacion=str(data.resultado_liquidacion),
            tipo_declaracion=data.tipo_declaracion,
        ),
    )


@router.post("/130", response_model=Generate130Response)
def generate_130(req: Generate130Request) -> Generate130Response:
    """Generate a Modelo 130 BOE file (quarterly IRPF advance)."""
    log.info("generate_130_request", year=req.year, quarter=req.quarter, from_quipu=req.from_quipu)
    settings = _get_settings()

    if req.from_quipu:
        if not settings.quipu_key:
            raise HTTPException(status_code=400, detail="Quipu credentials not configured")
        quipu_cfg = QuipuConfig(api_key=settings.quipu_key, api_secret=settings.quipu_secret)
        q_num = int(req.quarter[0])
        total_income = Decimal("0")
        total_expenses = Decimal("0")
        with QuipuClient(quipu_cfg) as client:
            for q in range(1, q_num + 1):
                totals = client.get_quarterly_totals(req.year, q)
                total_income += totals.total_income_gross
                total_expenses += totals.total_expenses_gross

        data = Modelo130Data(
            nif=settings.declarant_nif,
            apellidos=settings.declarant_apellidos,
            nombre=settings.declarant_nombre,
            exercise=req.year,
            period=req.quarter,
            ingresos=total_income,
            gastos=total_expenses,
            pagos_anteriores=Decimal(req.prev_payments),
            cuenta_iban=settings.iban,
        )
    else:
        data = Modelo130Data(
            nif=settings.declarant_nif,
            apellidos=settings.declarant_apellidos,
            nombre=settings.declarant_nombre,
            exercise=req.year,
            period=req.quarter,
            ingresos=Decimal(req.ingresos),
            gastos=Decimal(req.gastos),
            pagos_anteriores=Decimal(req.prev_payments),
            retenciones=Decimal(req.retenciones),
            cuenta_iban=settings.iban,
        )

    boe = generate_130_boe(data)

    log.info(
        "generate_130_complete",
        year=req.year,
        quarter=req.quarter,
        resultado=str(data.resultado),
        tipo=data.tipo_declaracion,
    )

    return Generate130Response(
        boe_content=boe,
        summary=Summary130(
            ingresos=str(data.ingresos),
            gastos=str(data.gastos),
            rendimiento_neto=str(data.rendimiento_neto),
            pago_20_pct=str(data.pago_20_pct),
            pagos_anteriores=str(data.pagos_anteriores),
            resultado=str(data.resultado),
            tipo_declaracion=data.tipo_declaracion,
        ),
    )
