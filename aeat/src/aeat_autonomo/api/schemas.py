"""Pydantic request/response models for the API."""

from __future__ import annotations

from decimal import Decimal, InvalidOperation
from typing import Annotated, Literal

from pydantic import BaseModel, Field, field_validator

Quarter = Literal["1T", "2T", "3T", "4T"]

# Year bounds — AEAT electronic filing started in 2002
MIN_YEAR = 2002
MAX_YEAR = 2100


def _validate_decimal(v: str, field_name: str) -> str:
    """Validate that a string is a valid decimal amount."""
    try:
        d = Decimal(v)
    except InvalidOperation:
        raise ValueError(f"{field_name}: '{v}' is not a valid decimal number")
    if d < 0:
        raise ValueError(f"{field_name}: amount cannot be negative")
    return v


def _validate_year(v: int) -> int:
    if v < MIN_YEAR or v > MAX_YEAR:
        raise ValueError(f"year must be between {MIN_YEAR} and {MAX_YEAR}")
    return v


# --- Generate ---

class Generate303Request(BaseModel):
    year: int = Field(..., examples=[2026])
    quarter: Quarter = Field(..., examples=["1T"])
    from_quipu: bool = Field(False, description="Fetch data from Quipu automatically.")
    # Manual data (ignored if from_quipu=True)
    base_21: str = "0"
    cuota_21: str = "0"
    base_10: str = "0"
    cuota_10: str = "0"
    base_4: str = "0"
    cuota_4: str = "0"
    base_deducible_interior: str = "0"
    cuota_deducible_interior: str = "0"

    @field_validator("year")
    @classmethod
    def check_year(cls, v: int) -> int:
        return _validate_year(v)

    @field_validator(
        "base_21", "cuota_21", "base_10", "cuota_10", "base_4", "cuota_4",
        "base_deducible_interior", "cuota_deducible_interior",
    )
    @classmethod
    def check_amounts(cls, v: str, info: object) -> str:
        return _validate_decimal(v, info.field_name)  # type: ignore[attr-defined]


class Generate303Response(BaseModel):
    boe_content: str
    summary: Summary303


class Summary303(BaseModel):
    iva_devengado: str
    iva_deducible: str
    resultado: str
    resultado_liquidacion: str
    tipo_declaracion: str


class Generate130Request(BaseModel):
    year: int = Field(..., examples=[2026])
    quarter: Quarter = Field(..., examples=["1T"])
    from_quipu: bool = Field(False, description="Fetch data from Quipu automatically.")
    prev_payments: str = Field("0", description="Previous quarters' advance payments this year.")
    # Manual data (ignored if from_quipu=True)
    ingresos: str = "0"
    gastos: str = "0"
    retenciones: str = "0"

    @field_validator("year")
    @classmethod
    def check_year(cls, v: int) -> int:
        return _validate_year(v)

    @field_validator("prev_payments", "ingresos", "gastos", "retenciones")
    @classmethod
    def check_amounts(cls, v: str, info: object) -> str:
        return _validate_decimal(v, info.field_name)  # type: ignore[attr-defined]


class Generate130Response(BaseModel):
    boe_content: str
    summary: Summary130


class Summary130(BaseModel):
    ingresos: str
    gastos: str
    rendimiento_neto: str
    pago_20_pct: str
    pagos_anteriores: str
    resultado: str
    tipo_declaracion: str


# --- Submit ---

class SubmitRequest(BaseModel):
    year: int = Field(..., examples=[2026])
    quarter: Quarter = Field(..., examples=["1T"])
    boe_content: str = Field(..., min_length=1, description="BOE flat file content.")
    nrc: str = Field("", description="NRC payment reference (required for tipo=Ingreso).")
    dry_run: bool = Field(False, description="Validate only, don't submit.")

    @field_validator("year")
    @classmethod
    def check_year(cls, v: int) -> int:
        return _validate_year(v)


class SubmitResponse(BaseModel):
    success: bool
    csv: str = ""
    justificante: str = ""
    fecha: str = ""
    hora: str = ""
    pdf_url: str = ""
    pdf_base64: str = ""
    warnings: list[str] = []
    errors: list[str] = []


# --- Quipu ---

class QuipuTotalsResponse(BaseModel):
    year: int
    quarter: str
    total_income_gross: str
    total_vat_collected: str
    total_expenses_gross: str
    total_vat_deductible: str
    net_vat: str
    net_income: str


# --- Simulate ---

class SimulateQuarterResult(BaseModel):
    quarter: str
    quipu: QuipuTotalsResponse
    modelo_303: Summary303
    modelo_130: Summary130


class SimulateResponse(BaseModel):
    year: int
    quarters: list[SimulateQuarterResult]
    annual_summary: AnnualSummary


class AnnualSummary(BaseModel):
    total_income: str
    total_expenses: str
    net_income: str
    total_vat_collected: str
    total_vat_deducted: str
    net_vat_paid: str
    irpf_advance_paid: str


# --- Health ---

class HealthResponse(BaseModel):
    status: str = "ok"
    config_loaded: bool
    certificate_configured: bool
    quipu_configured: bool
    testing_mode: bool


# --- Error ---

class ErrorResponse(BaseModel):
    error: str
    detail: str
