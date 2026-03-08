"""Modelo 130 — Pago fraccionado IRPF (estimación directa).

Generates BOE fixed-width files for quarterly IRPF advance payments.
Layout based on AEAT's diseño de registro DR130e15v12 (2015 v1.2).

File structure:
  - Wrapper record (DR 13000): <T1300AAAAPP0000>...<AUX>...</AUX>
  - Data record (DR 13001): 600 positions fixed-width
  - Closing tag: </T1300AAAAPP0000>

Key sections for a basic autónomo (estimación directa, no agriculture):
  - Section I: Casillas [01]-[07] — income, expenses, 20% payment
  - Section III: Casillas [12]-[19] — total liquidación and resultado

Reference: https://sede.agenciatributaria.gob.es/static_files/Sede/Disenyo_registro/DR_100_199/archivos/DR130e15v12.xls
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal, ROUND_HALF_UP


def _cents(amount: Decimal) -> int:
    """Convert a Decimal euro amount to integer cents."""
    return int((amount * 100).to_integral_value(rounding=ROUND_HALF_UP))


def _num(amount: int, length: int = 17) -> str:
    """Format unsigned numeric field: right-justified, zero-padded."""
    return str(max(0, amount)).rjust(length, "0")[:length]


def _signed(amount: int, length: int = 17) -> str:
    """Format signed numeric field (type N).

    First position: blank for positive/zero, 'N' for negative.
    Remaining positions: absolute value, right-justified, zero-padded.
    """
    if amount < 0:
        return "N" + str(abs(amount)).rjust(length - 1, "0")[:length - 1]
    return " " + str(amount).rjust(length - 1, "0")[:length - 1]


def _an(value: str, length: int) -> str:
    """Format alphanumeric field: left-justified, space-padded, uppercase."""
    return value.upper().ljust(length)[:length]


@dataclass
class Modelo130Data:
    """Data needed to generate a Modelo 130 declaration.

    All monetary amounts are year-to-date (acumulado), except where noted.
    """

    # Declarant
    nif: str
    apellidos: str  # Surnames (max 60 chars)
    nombre: str  # First name (max 20 chars)
    exercise: int  # e.g., 2026
    period: str  # "1T", "2T", "3T", "4T"

    # Section I: Actividades económicas en estimación directa
    # Casilla [01]: Ingresos computables (year-to-date computable income)
    ingresos: Decimal = Decimal("0")
    # Casilla [02]: Gastos fiscalmente deducibles (year-to-date deductible expenses)
    gastos: Decimal = Decimal("0")
    # Casilla [05]: Pagos fraccionados de trimestres anteriores del mismo ejercicio
    pagos_anteriores: Decimal = Decimal("0")
    # Casilla [06]: Retenciones e ingresos a cuenta soportados
    retenciones: Decimal = Decimal("0")

    # Section III: Deductions
    # Casilla [13]: Minoración deducción art. 110.3 Reglamento IRPF
    minoracion_art110: Decimal = Decimal("0")
    # Casilla [15]: Resultados negativos ejercicios anteriores
    resultados_negativos_ant: Decimal = Decimal("0")
    # Casilla [16]: Deducción vivienda habitual (only until legacy cutoff)
    deduccion_vivienda: Decimal = Decimal("0")
    # Casilla [18]: Resultado liquidaciones anteriores (only complementaria)
    resultado_anterior: Decimal = Decimal("0")

    # Complementaria
    complementaria: bool = False
    justificante_anterior: str = ""

    # Payment
    cuenta_iban: str = ""

    # --- Computed properties following the official casilla chain ---

    @property
    def rendimiento_neto(self) -> Decimal:
        """Casilla [03]: [01] - [02]. Can be negative."""
        return self.ingresos - self.gastos

    @property
    def pago_20_pct(self) -> Decimal:
        """Casilla [04]: 20% of [03]. Zero if [03] is negative."""
        rn = self.rendimiento_neto
        if rn <= 0:
            return Decimal("0")
        return (rn * Decimal("0.20")).quantize(Decimal("0.01"), rounding=ROUND_HALF_UP)

    @property
    def pago_fraccionado_previo(self) -> Decimal:
        """Casilla [07]: [04] - [05] - [06]. Can be negative."""
        return self.pago_20_pct - self.pagos_anteriores - self.retenciones

    @property
    def suma_pagos(self) -> Decimal:
        """Casilla [12]: [07] + [11]. For basic autónomo, [11]=0."""
        return self.pago_fraccionado_previo

    @property
    def diferencia(self) -> Decimal:
        """Casilla [14]: [12] - [13]."""
        return self.suma_pagos - self.minoracion_art110

    @property
    def total(self) -> Decimal:
        """Casilla [17]: [14] - [15] - [16]."""
        return self.diferencia - self.resultados_negativos_ant - self.deduccion_vivienda

    @property
    def resultado(self) -> Decimal:
        """Casilla [19]: [17] - [18]. Final result."""
        return self.total - self.resultado_anterior

    @property
    def tipo_declaracion(self) -> str:
        """Declaration type based on result."""
        if self.resultado > 0:
            return "I"  # Ingreso
        return "N"  # Negativa


def generate_130_page_record(data: Modelo130Data) -> str:
    """Generate the DR 13001 data record (600 positions fixed-width).

    Field positions are 1-based per the official diseño de registro.
    """
    buf = [" "] * 600

    def _put(pos: int, value: str) -> None:
        """Write value into buffer at 1-based position."""
        start = pos - 1
        for i, ch in enumerate(value):
            if start + i < 600:
                buf[start + i] = ch

    # --- Header / Identification ---
    _put(1, "<T")  # pos 1-2
    _put(3, "130")  # pos 3-5: model
    _put(6, "01")  # pos 6-7: page number
    _put(8, "000>")  # pos 8-11: constant
    # pos 12: complementary page indicator (blank)
    _put(13, data.tipo_declaracion)  # pos 13: declaration type
    _put(14, _an(data.nif, 9))  # pos 14-22: NIF
    _put(23, _an(data.apellidos, 60))  # pos 23-82: surnames
    _put(83, _an(data.nombre, 20))  # pos 83-102: first name
    _put(103, str(data.exercise))  # pos 103-106: exercise year
    _put(107, _an(data.period, 2))  # pos 107-108: period

    # --- Section I: Estimación directa ---
    _put(109, _num(_cents(data.ingresos)))  # [01] pos 109-125
    _put(126, _num(_cents(data.gastos)))  # [02] pos 126-142
    _put(143, _signed(_cents(data.rendimiento_neto)))  # [03] pos 143-159
    _put(160, _num(_cents(data.pago_20_pct)))  # [04] pos 160-176
    _put(177, _num(_cents(data.pagos_anteriores)))  # [05] pos 177-193
    _put(194, _num(_cents(data.retenciones)))  # [06] pos 194-210
    _put(211, _signed(_cents(data.pago_fraccionado_previo)))  # [07] pos 211-227

    # --- Section II: Agricultural (zeros for basic autónomo) ---
    _put(228, _num(0))  # [08] pos 228-244
    _put(245, _num(0))  # [09] pos 245-261
    _put(262, _num(0))  # [10] pos 262-278
    _put(279, _signed(0))  # [11] pos 279-295

    # --- Section III: Total liquidación ---
    _put(296, _num(_cents(data.suma_pagos)))  # [12] pos 296-312
    _put(313, _num(_cents(data.minoracion_art110)))  # [13] pos 313-329
    _put(330, _signed(_cents(data.diferencia)))  # [14] pos 330-346
    _put(347, _num(_cents(data.resultados_negativos_ant)))  # [15] pos 347-363
    _put(364, _num(_cents(data.deduccion_vivienda)))  # [16] pos 364-380
    _put(381, _signed(_cents(data.total)))  # [17] pos 381-397
    _put(398, _num(_cents(data.resultado_anterior)))  # [18] pos 398-414
    _put(415, _signed(_cents(data.resultado)))  # [19] pos 415-431

    # --- Tail ---
    _put(432, "X" if data.complementaria else " ")  # pos 432: complementaria
    if data.complementaria and data.justificante_anterior:
        _put(433, _an(data.justificante_anterior, 13))  # pos 433-445
    _put(446, _an(data.cuenta_iban.replace(" ", ""), 34))  # pos 446-479: IBAN
    # pos 480-575: reserved (blanks)
    # pos 576-588: reserved sello electrónico (blanks)
    _put(589, "</T13001000>")  # pos 589-600: end tag

    return "".join(buf)


def generate_130_wrapper(data: Modelo130Data) -> str:
    """Generate the DR 13000 wrapper record."""
    year = str(data.exercise)
    period = data.period

    # Build wrapper header (positions 1-329 of wrapper)
    header = f"<T1300{year}{period}0000>"
    aux_start = "<AUX>"
    aux_content = " " * 70  # pos 23-92: reserved blanks
    aux_version = "    "  # pos 93-96: software version (optional)
    aux_reserved = "    "  # pos 97-100: reserved
    aux_dev_nif = "         "  # pos 101-109: developer NIF (optional)
    aux_reserved2 = " " * 213  # pos 110-322: reserved
    aux_end = "</AUX>"

    wrapper_header = (
        header + aux_start + aux_content + aux_version
        + aux_reserved + aux_dev_nif + aux_reserved2 + aux_end
    )

    # Page record
    page_record = generate_130_page_record(data)

    # Closing tag
    closing = f"</T1300{year}{period}0000>"

    return wrapper_header + page_record + closing


def generate_130_boe(data: Modelo130Data) -> str:
    """Generate a complete Modelo 130 BOE file.

    Returns the full file content ready for submission via
    Servicios Comunes or file upload.
    """
    return generate_130_wrapper(data)
