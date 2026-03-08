"""Pydantic request/response models for the API."""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


Quarter = Literal["1T", "2T", "3T", "4T"]


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
    boe_content: str = Field(..., description="BOE flat file content.")
    nrc: str = Field("", description="NRC payment reference (required for tipo=Ingreso).")
    dry_run: bool = Field(False, description="Validate only, don't submit.")


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
