"""Modelo 130 — Pago fraccionado IRPF (estimación directa).

Generates BOE data for quarterly IRPF advance payments for autónomos
using the estimación directa (normal or simplificada) method.

Key sections:
- Section I: Actividades económicas en estimación directa
  - Box 01: Net income from start of year through current quarter
  - Box 02: 20% of box 01
  - Box 03: Previous quarters' advance payments this year
  - Box 04: Box 02 - Box 03 (amount to pay)
- Section V: Resultado
  - Box 18: Total advance payment (= box 04 for most autónomos)
  - Box 19: Deductions (e.g., maternidad/paternidad)
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal, ROUND_HALF_UP

from .boe import format_amount, format_nif


def _cents(amount: Decimal) -> int:
    """Convert a Decimal euro amount to integer cents."""
    return int((amount * 100).to_integral_value(rounding=ROUND_HALF_UP))


@dataclass
class Modelo130Data:
    """Data needed to generate a Modelo 130 declaration."""

    # Declarant
    nif: str
    name: str
    exercise: int  # e.g., 2026
    period: str  # "1T", "2T", "3T", "4T"

    # Section I: Actividades económicas en estimación directa
    # Box 01: Rendimiento neto acumulado desde inicio del ejercicio
    rendimiento_neto_acumulado: Decimal = Decimal("0")

    # Box 03: Pagos fraccionados acumulados de trimestres anteriores
    pagos_anteriores: Decimal = Decimal("0")

    # Box 05: Retenciones e ingresos a cuenta soportados
    retenciones: Decimal = Decimal("0")

    # Section V: Additional deductions
    # Box 19: Deducción art. 80 bis (maternidad, etc.)
    deduccion_maternidad: Decimal = Decimal("0")

    # Payment details
    cuenta_iban: str = ""
    tipo_declaracion: str = "I"  # I=ingreso, N=negativa

    @property
    def pago_fraccionado_bruto(self) -> Decimal:
        """Box 02: 20% of accumulated net income."""
        return (self.rendimiento_neto_acumulado * Decimal("0.20")).quantize(
            Decimal("0.01"), rounding=ROUND_HALF_UP
        )

    @property
    def pago_fraccionado_neto(self) -> Decimal:
        """Box 04: Box 02 - Box 03 (net advance payment this quarter)."""
        result = self.pago_fraccionado_bruto - self.pagos_anteriores
        return max(result, Decimal("0"))

    @property
    def total_liquidacion(self) -> Decimal:
        """Box 18: Total = section I result - retenciones."""
        return self.pago_fraccionado_neto - self.retenciones

    @property
    def resultado(self) -> Decimal:
        """Box 19: Final result after deductions."""
        result = self.total_liquidacion - self.deduccion_maternidad
        return result


def generate_130_fields(data: Modelo130Data) -> dict[str, str]:
    """Generate the field-value mapping for Modelo 130."""
    fields: dict[str, str] = {}

    # Identification
    fields["NIF"] = format_nif(data.nif)
    fields["APELLIDOS_NOMBRE"] = data.name[:60].upper()
    fields["EJERCICIO"] = str(data.exercise)
    fields["PERIODO"] = data.period
    fields["TIPO_DECLARACION"] = data.tipo_declaracion

    # Section I: Actividades económicas en estimación directa
    fields["01"] = format_amount(_cents(data.rendimiento_neto_acumulado))
    fields["02"] = format_amount(_cents(data.pago_fraccionado_bruto))
    fields["03"] = format_amount(_cents(data.pagos_anteriores))
    fields["04"] = format_amount(_cents(data.pago_fraccionado_neto))
    fields["05"] = format_amount(_cents(data.retenciones))

    # Section V: Resultado
    fields["18"] = format_amount(_cents(data.total_liquidacion))
    fields["19"] = format_amount(_cents(data.resultado))

    if data.cuenta_iban:
        fields["IBAN"] = data.cuenta_iban.replace(" ", "")

    return fields


def generate_130_boe(data: Modelo130Data) -> str:
    """Generate a Modelo 130 in AEAT's presentación directa format.

    Same tagged format as 303 for Servicios Comunes submission.
    """
    fields = generate_130_fields(data)

    lines: list[str] = []
    lines.append(f"<T1300{data.exercise}{data.period}>")
    lines.append("<AUX>")

    # Identification
    lines.append(f"  <D00>{fields['NIF']}")
    lines.append(f"  <D02>{fields['APELLIDOS_NOMBRE']}")
    lines.append(f"  <D03>{data.exercise}")
    lines.append(f"  <D04>{data.period}")
    lines.append(f"  <D05>{data.tipo_declaracion}")

    # Section I
    for casilla in ["01", "02", "03", "04", "05"]:
        lines.append(f"  <{casilla}>{fields[casilla]}")

    # Section V: Resultado
    lines.append(f"  <18>{fields['18']}")
    lines.append(f"  <19>{fields['19']}")

    if data.cuenta_iban:
        lines.append(f"  <IBAN>{fields['IBAN']}")

    lines.append("</AUX>")
    lines.append(f"</T1300{data.exercise}{data.period}>")

    return "\n".join(lines)
