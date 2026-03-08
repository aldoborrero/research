"""Modelo 303 — IVA Autoliquidación trimestral.

Generates BOE fixed-width files for quarterly VAT self-assessment.
Layout based on AEAT's diseño de registro DR303e26v101 (exercise 2026).

File structure (tagged pages, concatenated as a single string):
  - Wrapper: <T3030YYYYPP0000> <AUX>...</AUX>
  - Page 01 (Sub01): <T30301000>...</T30301000> — IVA devengado + deducible
  - Page 03 (Sub03): <T30303000>...</T30303000> — Info adicional + resultado
  - Page 04 (Sub04): Only for 4T/12 — annual summary fields (omitted for Q1-Q3)
  - Closing: </T3030YYYYPP0000>

Numeric format (17-pos monetary fields):
  - Sign: first char blank (positive) or 'N' (negative)
  - Integer: next 15 chars, right-justified, zero-padded
  - Decimal: last 2 chars (centimos, no separator)

Percentage format (5-pos): 3 integer + 2 decimal, no separator.
  e.g., 21% = "02100", 10% = "01000", 4% = "00400"

Reference: https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro/modelos-300-399.html
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal

from .boe import an as _an
from .boe import bool_yn as _bool_yn
from .boe import cents as _cents
from .boe import num as _num
from .boe import pct as _pct
from .boe import signed as _signed
from .boe import validate_iban, validate_nif


@dataclass
class Modelo303Data:
    """Data needed to generate a Modelo 303 declaration.

    Covers the standard case for an autónomo filing quarterly
    under régimen general (not simplificado).
    """

    # Declarant
    nif: str
    nombre_razon: str  # Full name (apellidos + nombre), max 80 chars
    exercise: int  # e.g., 2026
    period: str  # "1T", "2T", "3T", "4T"

    # --- IVA Devengado (output / collected VAT) ---
    # Casillas [01]-[03]: Super-reduced rate 4%
    base_4: Decimal = Decimal("0")
    cuota_4: Decimal = Decimal("0")

    # Casillas [04]-[06]: Reduced rate 10%
    base_10: Decimal = Decimal("0")
    cuota_10: Decimal = Decimal("0")

    # Casillas [07]-[09]: General rate 21%
    base_21: Decimal = Decimal("0")
    cuota_21: Decimal = Decimal("0")

    # Casillas [10]-[11]: Intra-community acquisitions
    base_intracom: Decimal = Decimal("0")
    cuota_intracom: Decimal = Decimal("0")

    # Casillas [12]-[13]: Inversión sujeto pasivo
    base_inv_suj_pasivo: Decimal = Decimal("0")
    cuota_inv_suj_pasivo: Decimal = Decimal("0")

    # Casillas [14]-[15]: Modificación bases y cuotas
    base_modificacion: Decimal = Decimal("0")
    cuota_modificacion: Decimal = Decimal("0")

    # --- IVA Deducible (input / deductible VAT) ---
    # Casillas [28]-[29]: Op. interiores corrientes
    base_deducible_interior: Decimal = Decimal("0")
    cuota_deducible_interior: Decimal = Decimal("0")

    # Casillas [30]-[31]: Bienes de inversión
    base_deducible_inversion: Decimal = Decimal("0")
    cuota_deducible_inversion: Decimal = Decimal("0")

    # Casillas [32]-[33]: Importaciones corrientes
    base_deducible_import_corr: Decimal = Decimal("0")
    cuota_deducible_import_corr: Decimal = Decimal("0")

    # Casillas [34]-[35]: Importaciones inversión
    base_deducible_import_inv: Decimal = Decimal("0")
    cuota_deducible_import_inv: Decimal = Decimal("0")

    # Casillas [36]-[37]: Adquisiciones intracom. corrientes
    base_deducible_intracom_corr: Decimal = Decimal("0")
    cuota_deducible_intracom_corr: Decimal = Decimal("0")

    # Casillas [38]-[39]: Adquisiciones intracom. inversión
    base_deducible_intracom_inv: Decimal = Decimal("0")
    cuota_deducible_intracom_inv: Decimal = Decimal("0")

    # Casillas [40]-[41]: Rectificación deducciones
    base_rectificacion: Decimal = Decimal("0")
    cuota_rectificacion: Decimal = Decimal("0")

    # Casilla [42]: Compensaciones régimen especial
    cuota_compensaciones: Decimal = Decimal("0")
    # Casilla [43]: Regularización inversiones
    cuota_reg_inversiones: Decimal = Decimal("0")
    # Casilla [44]: Regularización prorrata
    cuota_reg_prorrata: Decimal = Decimal("0")

    # --- Page 03 fields ---
    # Casilla [59]: Entregas intracomunitarias
    entregas_intracom: Decimal = Decimal("0")
    # Casilla [60]: Exportaciones
    exportaciones: Decimal = Decimal("0")

    # Casilla [78]: Cuotas a compensar de periodos anteriores
    compensacion_anterior: Decimal = Decimal("0")
    # Casilla [68]: Regularización anual (only 4T)
    regularizacion_anual: Decimal = Decimal("0")
    # Casilla [70]: A deducir periodos anteriores (only complementaria)
    deducir_periodos_ant: Decimal = Decimal("0")

    # Options
    regimen_simplificado: bool = False
    criterio_caja: bool = False
    destinatario_criterio_caja: bool = False
    devolucion_mensual: bool = False
    prorrata_especial: bool = False
    sii_voluntario: bool = False
    complementaria: bool = False
    justificante_anterior: str = ""
    sin_actividad: bool = False

    # Payment
    cuenta_iban: str = ""

    def __post_init__(self) -> None:
        validate_nif(self.nif)
        if self.cuenta_iban:
            self.cuenta_iban = validate_iban(self.cuenta_iban)

    # --- Computed properties (casilla chain) ---

    @property
    def total_cuota_devengada(self) -> Decimal:
        """Casilla [27]: Total IVA devengado."""
        return (
            self.cuota_4 + self.cuota_10 + self.cuota_21
            + self.cuota_intracom + self.cuota_inv_suj_pasivo
            + self.cuota_modificacion
        )

    @property
    def total_a_deducir(self) -> Decimal:
        """Casilla [45]: Total IVA deducible."""
        return (
            self.cuota_deducible_interior + self.cuota_deducible_inversion
            + self.cuota_deducible_import_corr + self.cuota_deducible_import_inv
            + self.cuota_deducible_intracom_corr + self.cuota_deducible_intracom_inv
            + self.cuota_rectificacion + self.cuota_compensaciones
            + self.cuota_reg_inversiones + self.cuota_reg_prorrata
        )

    @property
    def resultado_regimen_general(self) -> Decimal:
        """Casilla [46]: [27] - [45]."""
        return self.total_cuota_devengada - self.total_a_deducir

    @property
    def suma_resultados(self) -> Decimal:
        """Casilla [64]: For basic autónomo, equals [46]."""
        return self.resultado_regimen_general

    @property
    def atribuible_estado(self) -> Decimal:
        """Casilla [66]: 100% of [64] for standard declarations."""
        return self.suma_resultados

    @property
    def resultado(self) -> Decimal:
        """Casilla [69]: Resultado = [66] + [77] - [78] + [68]."""
        return (
            self.atribuible_estado
            - self.compensacion_anterior
            + self.regularizacion_anual
        )

    @property
    def resultado_liquidacion(self) -> Decimal:
        """Casilla [71]: [69] - [70]. Final amount."""
        return self.resultado - self.deducir_periodos_ant

    @property
    def tipo_declaracion(self) -> str:
        """Determine declaration type from result."""
        r = self.resultado_liquidacion
        if r > 0:
            return "I"  # Ingreso
        if r < 0:
            if self.period == "4T":
                return "D"  # Devolución (only in last quarter)
            return "C"  # Compensar
        if self.sin_actividad:
            return "N"  # Sin actividad
        return "N"  # Negativa / zero


def _generate_page01(data: Modelo303Data) -> str:
    """Generate Page 01 (Sub01): IVA devengado + deducible."""
    buf: list[str] = []

    def _w(value: str) -> None:
        buf.append(value)

    # Header
    _w("<T")  # 2
    _w("303")  # 3
    _w("01000")  # 5
    _w(">")  # 1

    # Identification
    _w(" ")  # Complementary page indicator (blank)
    _w(data.tipo_declaracion)  # 1: tipo declaración
    _w(_an(data.nif, 9))  # 9: NIF
    _w(_an(data.nombre_razon, 80))  # 80: name
    _w(str(data.exercise))  # 4: year
    _w(_an(data.period, 2))  # 2: period

    # Flags
    _w("2")  # Tributación foral: NO
    _w(_bool_yn(data.devolucion_mensual))  # Devolución mensual
    if data.regimen_simplificado:
        _w("1")  # Solo simplificado
    else:
        _w("3")  # No simplificado
    _w("2")  # Autoliquidación conjunta: NO
    _w(_bool_yn(data.criterio_caja))
    _w(_bool_yn(data.destinatario_criterio_caja))
    _w(_bool_yn(data.prorrata_especial))
    _w("2")  # Revocación prorrata: NO
    _w("2")  # Concurso acreedores: NO
    _w(" " * 8)  # Fecha auto concurso (blanks)
    _w(" ")  # Auto concurso en periodo (blank)
    _w(_bool_yn(data.sii_voluntario))
    _w("0")  # Exonerado 390 (handle in 4T)
    _w("0")  # Volumen operaciones

    # --- IVA Devengado ---
    # [01]-[03]: 4%
    _w(_signed(_cents(data.base_4)))
    _w(_pct(Decimal("4")))
    _w(_signed(_cents(data.cuota_4)))
    # [04]-[06]: 10%
    _w(_signed(_cents(data.base_10)))
    _w(_pct(Decimal("10")))
    _w(_signed(_cents(data.cuota_10)))
    # [07]-[09]: 21%
    _w(_signed(_cents(data.base_21)))
    _w(_pct(Decimal("21")))
    _w(_signed(_cents(data.cuota_21)))
    # [10]-[11]: Adquisiciones intracomunitarias
    _w(_signed(_cents(data.base_intracom)))
    _w(_signed(_cents(data.cuota_intracom)))
    # [12]-[13]: Inversión sujeto pasivo
    _w(_signed(_cents(data.base_inv_suj_pasivo)))
    _w(_signed(_cents(data.cuota_inv_suj_pasivo)))
    # [14]-[15]: Modificación bases y cuotas
    _w(_signed(_cents(data.base_modificacion)))
    _w(_signed(_cents(data.cuota_modificacion)))
    # [16]-[18]: Recargo equivalencia 0.5% (zeros for autónomo)
    _w(_signed(0))  # base
    _w(_pct(Decimal("0.5")))  # tipo
    _w(_signed(0))  # cuota
    # [19]-[21]: Recargo equivalencia 1.4%
    _w(_signed(0))
    _w(_pct(Decimal("1.4")))
    _w(_signed(0))
    # [22]-[24]: Recargo equivalencia 5.2%
    _w(_signed(0))
    _w(_pct(Decimal("5.2")))
    _w(_signed(0))
    # [25]-[26]: Modificación recargo
    _w(_signed(0))
    _w(_signed(0))
    # [27]: Total cuota devengada
    _w(_signed(_cents(data.total_cuota_devengada)))

    # --- IVA Deducible ---
    # [28]-[29]: Op. interiores corrientes
    _w(_signed(_cents(data.base_deducible_interior)))
    _w(_signed(_cents(data.cuota_deducible_interior)))
    # [30]-[31]: Bienes de inversión
    _w(_signed(_cents(data.base_deducible_inversion)))
    _w(_signed(_cents(data.cuota_deducible_inversion)))
    # [32]-[33]: Importaciones corrientes
    _w(_signed(_cents(data.base_deducible_import_corr)))
    _w(_signed(_cents(data.cuota_deducible_import_corr)))
    # [34]-[35]: Importaciones inversión
    _w(_signed(_cents(data.base_deducible_import_inv)))
    _w(_signed(_cents(data.cuota_deducible_import_inv)))
    # [36]-[37]: Adquisiciones intracom. corrientes
    _w(_signed(_cents(data.base_deducible_intracom_corr)))
    _w(_signed(_cents(data.cuota_deducible_intracom_corr)))
    # [38]-[39]: Adquisiciones intracom. inversión
    _w(_signed(_cents(data.base_deducible_intracom_inv)))
    _w(_signed(_cents(data.cuota_deducible_intracom_inv)))
    # [40]-[41]: Rectificación deducciones
    _w(_signed(_cents(data.base_rectificacion)))
    _w(_signed(_cents(data.cuota_rectificacion)))
    # [42]: Compensaciones régimen especial
    _w(_signed(_cents(data.cuota_compensaciones)))
    # [43]: Regularización inversiones
    _w(_signed(_cents(data.cuota_reg_inversiones)))
    # [44]: Regularización prorrata
    _w(_signed(_cents(data.cuota_reg_prorrata)))
    # [45]: Total a deducir
    _w(_signed(_cents(data.total_a_deducir)))
    # [46]: Resultado régimen general
    _w(_signed(_cents(data.resultado_regimen_general)))

    # Reserved AEAT (600 blanks)
    _w(" " * 600)
    # Sello electrónico (13 blanks)
    _w(" " * 13)
    # End tag
    _w("</T30301000>")

    return "".join(buf)


def _generate_page03(data: Modelo303Data) -> str:
    """Generate Page 03 (Sub03): Información adicional + resultado."""
    buf: list[str] = []

    def _w(value: str) -> None:
        buf.append(value)

    # Header
    _w("<T")
    _w("303")
    _w("03000")
    _w(">")

    # Casillas informativas
    _w(_signed(_cents(data.entregas_intracom)))  # [59]
    _w(_signed(_cents(data.exportaciones)))  # [60]
    _w(_signed(0))  # [120]: Op. no sujetas localización
    _w(_signed(0))  # [122]: Op. inversión sujeto pasivo
    _w(_signed(0))  # [123]: OSS no sujetas
    _w(_signed(0))  # [124]: OSS sujetas
    _w(_signed(0))  # [62]: Importes devengados art. 75 — base
    _w(_signed(0))  # [63]: Importes devengados art. 75 — cuota
    _w(_signed(0))  # [74]: Cuotas criterio caja — base
    _w(_signed(0))  # [75]: Cuotas criterio caja — cuota
    _w(_signed(0))  # [76]: Regularización cuotas art. 80

    # Resultado
    _w(_signed(_cents(data.suma_resultados)))  # [64]
    _w(_pct(Decimal("100")))  # [65]: % atribuible Estado = 100%
    _w(_signed(_cents(data.atribuible_estado)))  # [66]
    _w(_signed(0))  # [77]: IVA importación aduana pendiente
    _w(_signed(0))  # [110]: Cuotas a compensar pendientes
    _w(_signed(_cents(data.compensacion_anterior)))  # [78]
    _w(_signed(0))  # [87]: Cuotas compensar pendientes posteriores
    _w(_signed(_cents(data.regularizacion_anual)))  # [68]
    _w(_signed(_cents(data.resultado)))  # [69]
    _w(_signed(_cents(data.deducir_periodos_ant)))  # [70]
    _w(_signed(_cents(data.resultado_liquidacion)))  # [71]

    # Complementaria / sin actividad
    _w("X" if data.complementaria else " ")
    _w(_an(data.justificante_anterior, 13))
    _w("X" if data.sin_actividad else " ")

    # Bank details
    _w(_an("", 11))  # SWIFT-BIC (blank for domestic)
    _w(_an(data.cuenta_iban.replace(" ", ""), 34))  # IBAN
    _w(" " * 17)  # Reserved
    _w(" " * 70)  # Bank name
    _w(" " * 35)  # Bank address
    _w(" " * 30)  # Bank city
    _w(" " * 2)  # Bank country code
    _w("0")  # Marca SEPA

    # Reserved AEAT (600 blanks)
    _w(" " * 600)
    # End tag
    _w("</T30303000>")

    return "".join(buf)


def _generate_wrapper(data: Modelo303Data) -> str:
    """Generate the main envelope wrapper."""
    year = str(data.exercise)
    period = data.period

    buf: list[str] = []
    buf.append(f"<T3030{year}{period}0000>")
    buf.append("<AUX>")
    buf.append(" " * 70)  # Reserved
    buf.append("    ")  # Program version (4 chars, optional)
    buf.append("    ")  # Reserved
    buf.append(" " * 9)  # Developer NIF (optional)
    buf.append(" " * 213)  # Reserved
    buf.append("</AUX>")

    return "".join(buf)


def generate_303_boe(data: Modelo303Data) -> str:
    """Generate a complete Modelo 303 BOE file.

    Returns the full file content as a single string, ready for
    submission via Presentación Directa (F01 JSON field) or file upload.

    The F01 field must contain no CRLF or tabs — this function
    returns a single-line string by design.
    """
    parts = [
        _generate_wrapper(data),
        _generate_page01(data),
        _generate_page03(data),
        f"</T3030{data.exercise}{data.period}0000>",
    ]
    return "".join(parts)
