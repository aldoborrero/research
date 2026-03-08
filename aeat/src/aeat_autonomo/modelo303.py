"""Modelo 303 — IVA Autoliquidación trimestral.

Generates BOE fixed-width files for quarterly VAT self-assessment.
Layout based on AEAT's diseño de registro DR303 (exercise 2026+).

The 303 BOE file consists of:
- Page header record (type "<T")
- Page start ("<AUX>") — auxiliary identification record
- Field records — one line per casilla (box) in the form

For Servicios Comunes submission, the file format is a sequence of
field-value pairs following AEAT's "presentación directa" protocol.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from decimal import Decimal, ROUND_HALF_UP

from .boe import format_amount, format_nif


def _cents(amount: Decimal) -> int:
    """Convert a Decimal euro amount to integer cents."""
    return int((amount * 100).to_integral_value(rounding=ROUND_HALF_UP))


@dataclass
class Modelo303Data:
    """Data needed to generate a Modelo 303 declaration.

    This covers the simplified case for an autónomo filing quarterly.
    """

    # Declarant
    nif: str
    name: str
    exercise: int  # e.g., 2026
    period: str  # "1T", "2T", "3T", "4T"

    # Régimen general — IVA repercutido (collected VAT)
    # Box 01-03: operations at general rate (21%)
    base_21: Decimal = Decimal("0")
    vat_21: Decimal = Decimal("0")

    # Box 04-06: operations at reduced rate (10%)
    base_10: Decimal = Decimal("0")
    vat_10: Decimal = Decimal("0")

    # Box 07-09: operations at super-reduced rate (4%)
    base_4: Decimal = Decimal("0")
    vat_4: Decimal = Decimal("0")

    # IVA soportado deducible (deductible input VAT)
    # Box 28-29: domestic operations
    base_deductible_domestic: Decimal = Decimal("0")
    vat_deductible_domestic: Decimal = Decimal("0")

    # Box 32-33: investment goods
    base_deductible_investment: Decimal = Decimal("0")
    vat_deductible_investment: Decimal = Decimal("0")

    # Box 36-37: intra-community acquisitions (goods)
    base_intracom_goods: Decimal = Decimal("0")
    vat_intracom_goods: Decimal = Decimal("0")

    # Box 40-41: intra-community acquisitions (services)
    base_intracom_services: Decimal = Decimal("0")
    vat_intracom_services: Decimal = Decimal("0")

    # Additional fields
    compensacion_anterior: Decimal = Decimal("0")  # Box 67: previous quarter compensation
    cuenta_iban: str = ""  # IBAN for payment/refund

    # Type of declaration
    tipo_declaracion: str = "I"  # I=ingreso, D=devolución, N=negativa, C=compensar

    @property
    def total_vat_collected(self) -> Decimal:
        """Total IVA repercutido (boxes 03+06+09)."""
        return self.vat_21 + self.vat_10 + self.vat_4

    @property
    def total_base_collected(self) -> Decimal:
        """Total base imponible IVA repercutido."""
        return self.base_21 + self.base_10 + self.base_4

    @property
    def total_vat_deductible(self) -> Decimal:
        """Total IVA soportado deducible (boxes 29+33+37+41)."""
        return (
            self.vat_deductible_domestic
            + self.vat_deductible_investment
            + self.vat_intracom_goods
            + self.vat_intracom_services
        )

    @property
    def total_base_deductible(self) -> Decimal:
        """Total base deducible."""
        return (
            self.base_deductible_domestic
            + self.base_deductible_investment
            + self.base_intracom_goods
            + self.base_intracom_services
        )

    @property
    def diferencia(self) -> Decimal:
        """Box 65: IVA repercutido - IVA soportado."""
        return self.total_vat_collected - self.total_vat_deductible

    @property
    def resultado(self) -> Decimal:
        """Box 69: Final result = diferencia - compensación anterior."""
        return self.diferencia - self.compensacion_anterior


def generate_303_fields(data: Modelo303Data) -> dict[str, str]:
    """Generate the field-value mapping for Modelo 303.

    Returns a dict of casilla number → value, suitable for both
    BOE file generation and Servicios Comunes SOAP submission.
    """
    period_map = {"1T": "1T", "2T": "2T", "3T": "3T", "4T": "4T"}
    period = period_map.get(data.period, data.period)

    fields: dict[str, str] = {}

    # Identification
    fields["NIF"] = format_nif(data.nif)
    fields["DEESSION"] = str(data.exercise)
    fields["DEESSION_PERIODO"] = period
    fields["APELLIDOS_NOMBRE"] = data.name[:60].upper()
    fields["TIPO_DECLARACION"] = data.tipo_declaracion

    # --- IVA Devengado (collected / output VAT) ---
    # General rate 21%
    fields["01"] = format_amount(_cents(data.base_21))
    fields["02"] = "2100"  # Rate as percentage * 100
    fields["03"] = format_amount(_cents(data.vat_21))

    # Reduced rate 10%
    fields["04"] = format_amount(_cents(data.base_10))
    fields["05"] = "1000"
    fields["06"] = format_amount(_cents(data.vat_10))

    # Super-reduced rate 4%
    fields["07"] = format_amount(_cents(data.base_4))
    fields["08"] = "0400"
    fields["09"] = format_amount(_cents(data.vat_4))

    # Total collected
    fields["27"] = format_amount(_cents(data.total_vat_collected))

    # --- IVA Deducible (input / deductible VAT) ---
    # Domestic operations
    fields["28"] = format_amount(_cents(data.base_deductible_domestic))
    fields["29"] = format_amount(_cents(data.vat_deductible_domestic))

    # Investment goods
    fields["32"] = format_amount(_cents(data.base_deductible_investment))
    fields["33"] = format_amount(_cents(data.vat_deductible_investment))

    # Intra-community goods
    fields["36"] = format_amount(_cents(data.base_intracom_goods))
    fields["37"] = format_amount(_cents(data.vat_intracom_goods))

    # Intra-community services
    fields["40"] = format_amount(_cents(data.base_intracom_services))
    fields["41"] = format_amount(_cents(data.vat_intracom_services))

    # Total deductible
    fields["45"] = format_amount(_cents(data.total_vat_deductible))

    # --- Resultado ---
    fields["65"] = format_amount(_cents(data.diferencia))
    fields["67"] = format_amount(_cents(data.compensacion_anterior))
    fields["69"] = format_amount(_cents(data.resultado))

    # Payment / refund
    if data.cuenta_iban:
        fields["IBAN"] = data.cuenta_iban.replace(" ", "")

    return fields


def generate_303_boe(data: Modelo303Data) -> str:
    """Generate a Modelo 303 in AEAT's presentación directa format.

    This generates the <T and field lines used by Servicios Comunes
    for direct presentation of autoliquidaciones.

    The format is:
        <T303{exercise}{period}>
        <AUX>
        <field_id>{value}
        </AUX>
        </T303>
    """
    period = data.period
    fields = generate_303_fields(data)

    lines: list[str] = []
    lines.append(f"<T3030{data.exercise}{period}>")
    lines.append("<AUX>")

    # Identification block
    lines.append(f"  <D00>{fields['NIF']}")
    lines.append(f"  <D02>{fields['APELLIDOS_NOMBRE']}")
    lines.append(f"  <D03>{data.exercise}")
    lines.append(f"  <D04>{period}")
    lines.append(f"  <D05>{data.tipo_declaracion}")

    # IVA Devengado — Régimen general
    for casilla in ["01", "02", "03", "04", "05", "06", "07", "08", "09", "27"]:
        lines.append(f"  <{casilla}>{fields[casilla]}")

    # IVA Deducible
    for casilla in ["28", "29", "32", "33", "36", "37", "40", "41", "45"]:
        lines.append(f"  <{casilla}>{fields[casilla]}")

    # Resultado
    for casilla in ["65", "67", "69"]:
        lines.append(f"  <{casilla}>{fields[casilla]}")

    if data.cuenta_iban:
        lines.append(f"  <IBAN>{fields['IBAN']}")

    lines.append("</AUX>")
    lines.append(f"</T3030{data.exercise}{period}>")

    return "\n".join(lines)
