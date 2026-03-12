"""Tests for modelo130.py — Modelo 130 IRPF quarterly advance payment."""

from decimal import Decimal

import pytest

from aeat_autonomo.models.modelo_130 import (
    Modelo130Data,
    _an,
    _cents,
    _num,
    _signed,
    generate_130_boe,
    generate_130_page_record,
    generate_130_wrapper,
)


# ---------------------------------------------------------------------------
# Helper function tests (modelo130 has its own copies)
# ---------------------------------------------------------------------------


class TestCents130:
    def test_whole_euros(self):
        assert _cents(Decimal("100")) == 10000

    def test_with_centimos(self):
        assert _cents(Decimal("99.99")) == 9999

    def test_negative(self):
        assert _cents(Decimal("-50")) == -5000

    def test_rounding(self):
        assert _cents(Decimal("1.005")) == 101


class TestNum130:
    def test_positive(self):
        assert _num(500) == "00000000000000500"

    def test_negative_clamped(self):
        assert _num(-1) == "00000000000000000"


class TestSigned130:
    def test_positive(self):
        assert _signed(500) == " 0000000000000500"

    def test_negative(self):
        assert _signed(-500) == "N0000000000000500"

    def test_zero(self):
        result = _signed(0)
        assert result[0] == " "
        assert len(result) == 17


# ---------------------------------------------------------------------------
# Modelo130Data computed properties
# ---------------------------------------------------------------------------


def make_130(**kwargs) -> Modelo130Data:
    """Create Modelo130Data with sensible defaults."""
    defaults = dict(
        nif="12345678Z",
        apellidos="GARCIA LOPEZ",
        nombre="JUAN",
        exercise=2026,
        period="1T",
    )
    defaults.update(kwargs)
    return Modelo130Data(**defaults)


class TestModelo130DataComputedProperties:
    def test_rendimiento_neto_positive(self):
        d = make_130(ingresos=Decimal("10000"), gastos=Decimal("3000"))
        assert d.rendimiento_neto == Decimal("7000")

    def test_rendimiento_neto_negative(self):
        d = make_130(ingresos=Decimal("1000"), gastos=Decimal("5000"))
        assert d.rendimiento_neto == Decimal("-4000")

    def test_rendimiento_neto_zero(self):
        d = make_130(ingresos=Decimal("5000"), gastos=Decimal("5000"))
        assert d.rendimiento_neto == Decimal("0")

    def test_pago_20_pct_positive(self):
        d = make_130(ingresos=Decimal("10000"), gastos=Decimal("3000"))
        # 20% of 7000 = 1400
        assert d.pago_20_pct == Decimal("1400.00")

    def test_pago_20_pct_zero_when_negative(self):
        d = make_130(ingresos=Decimal("1000"), gastos=Decimal("5000"))
        assert d.pago_20_pct == Decimal("0")

    def test_pago_20_pct_zero_when_zero_income(self):
        d = make_130()
        assert d.pago_20_pct == Decimal("0")

    def test_pago_20_pct_rounding(self):
        # 20% of 1001 = 200.20
        d = make_130(ingresos=Decimal("1001"), gastos=Decimal("0"))
        assert d.pago_20_pct == Decimal("200.20")

    def test_pago_fraccionado_previo(self):
        d = make_130(
            ingresos=Decimal("10000"),
            gastos=Decimal("3000"),
            pagos_anteriores=Decimal("500"),
            retenciones=Decimal("200"),
        )
        # 20% of 7000 = 1400, minus 500 prior, minus 200 withholdings = 700
        assert d.pago_fraccionado_previo == Decimal("700.00")

    def test_pago_fraccionado_previo_negative(self):
        d = make_130(
            ingresos=Decimal("5000"),
            gastos=Decimal("3000"),
            pagos_anteriores=Decimal("500"),
            retenciones=Decimal("200"),
        )
        # 20% of 2000 = 400, minus 500 prior, minus 200 = -300
        assert d.pago_fraccionado_previo == Decimal("-300.00")

    def test_suma_pagos_equals_pago_fraccionado(self):
        d = make_130(ingresos=Decimal("10000"))
        assert d.suma_pagos == d.pago_fraccionado_previo

    def test_diferencia(self):
        d = make_130(
            ingresos=Decimal("10000"),
            minoracion_art110=Decimal("100"),
        )
        expected = d.suma_pagos - Decimal("100")
        assert d.diferencia == expected

    def test_total(self):
        d = make_130(
            ingresos=Decimal("10000"),
            resultados_negativos_ant=Decimal("50"),
            deduccion_vivienda=Decimal("30"),
        )
        expected = d.diferencia - Decimal("50") - Decimal("30")
        assert d.total == expected

    def test_resultado(self):
        d = make_130(
            ingresos=Decimal("10000"),
            resultado_anterior=Decimal("100"),
        )
        expected = d.total - Decimal("100")
        assert d.resultado == expected

    def test_full_chain(self):
        """Test the complete casilla chain with realistic Q2 values."""
        d = make_130(
            nif="12345678Z",
            apellidos="GARCIA LOPEZ",
            nombre="JUAN",
            exercise=2026,
            period="2T",
            ingresos=Decimal("20000"),  # Year-to-date income
            gastos=Decimal("8000"),  # Year-to-date expenses
            pagos_anteriores=Decimal("1200"),  # Q1 payment
            retenciones=Decimal("300"),
        )
        # rendimiento_neto = 20000 - 8000 = 12000
        assert d.rendimiento_neto == Decimal("12000")
        # pago_20_pct = 20% of 12000 = 2400
        assert d.pago_20_pct == Decimal("2400.00")
        # pago_fraccionado = 2400 - 1200 - 300 = 900
        assert d.pago_fraccionado_previo == Decimal("900.00")
        # resultado = 900 (no deductions/adjustments)
        assert d.resultado == Decimal("900.00")
        assert d.tipo_declaracion == "I"


class TestModelo130TipoDeclaracion:
    def test_ingreso(self):
        d = make_130(ingresos=Decimal("10000"))
        assert d.tipo_declaracion == "I"

    def test_negativa_zero(self):
        d = make_130()
        assert d.tipo_declaracion == "N"

    def test_negativa_negative_result(self):
        d = make_130(
            ingresos=Decimal("1000"),
            gastos=Decimal("5000"),
        )
        assert d.tipo_declaracion == "N"

    def test_negativa_when_prior_payments_exceed(self):
        d = make_130(
            ingresos=Decimal("10000"),
            gastos=Decimal("3000"),
            pagos_anteriores=Decimal("2000"),
        )
        # 20% of 7000 = 1400, minus 2000 = -600 → N
        assert d.tipo_declaracion == "N"


# ---------------------------------------------------------------------------
# BOE generation tests
# ---------------------------------------------------------------------------


class TestGenerate130PageRecord:
    def test_length_is_600(self):
        d = make_130()
        record = generate_130_page_record(d)
        assert len(record) == 600

    def test_starts_with_header_tag(self):
        d = make_130()
        record = generate_130_page_record(d)
        assert record[:11] == "<T13001000>"

    def test_ends_with_closing_tag(self):
        d = make_130()
        record = generate_130_page_record(d)
        assert record[588:600] == "</T13001000>"

    def test_nif_at_correct_position(self):
        d = make_130(nif="12345678Z")
        record = generate_130_page_record(d)
        # NIF at positions 14-22 (0-indexed: 13-21)
        nif_field = record[13:22]
        assert nif_field == "12345678Z"

    def test_apellidos_at_correct_position(self):
        d = make_130(apellidos="GARCIA LOPEZ")
        record = generate_130_page_record(d)
        # Surnames at positions 23-82 (0-indexed: 22-81)
        apellidos_field = record[22:82]
        assert apellidos_field.startswith("GARCIA LOPEZ")
        assert len(apellidos_field) == 60

    def test_nombre_at_correct_position(self):
        d = make_130(nombre="JUAN")
        record = generate_130_page_record(d)
        # First name at positions 83-102 (0-indexed: 82-101)
        nombre_field = record[82:102]
        assert nombre_field.startswith("JUAN")
        assert len(nombre_field) == 20

    def test_exercise_at_correct_position(self):
        d = make_130(exercise=2026)
        record = generate_130_page_record(d)
        # Exercise at positions 103-106 (0-indexed: 102-105)
        assert record[102:106] == "2026"

    def test_period_at_correct_position(self):
        d = make_130(period="3T")
        record = generate_130_page_record(d)
        # Period at positions 107-108 (0-indexed: 106-107)
        assert record[106:108] == "3T"

    def test_tipo_declaracion_at_correct_position(self):
        d = make_130(ingresos=Decimal("10000"))
        record = generate_130_page_record(d)
        # Type at position 13 (0-indexed: 12)
        assert record[12] == "I"

    def test_ingresos_at_correct_position(self):
        d = make_130(ingresos=Decimal("10000"))
        record = generate_130_page_record(d)
        # [01] at positions 109-125 (0-indexed: 108-124)
        field = record[108:125]
        assert field == "00000000001000000"  # 10000.00 in cents

    def test_gastos_at_correct_position(self):
        d = make_130(gastos=Decimal("3000"))
        record = generate_130_page_record(d)
        # [02] at positions 126-142 (0-indexed: 125-141)
        field = record[125:142]
        assert field == "00000000000300000"

    def test_rendimiento_neto_at_correct_position(self):
        d = make_130(ingresos=Decimal("10000"), gastos=Decimal("3000"))
        record = generate_130_page_record(d)
        # [03] at positions 143-159 (0-indexed: 142-158), signed
        field = record[142:159]
        assert field == " 0000000000700000"  # 7000.00 positive

    def test_rendimiento_neto_negative(self):
        d = make_130(ingresos=Decimal("1000"), gastos=Decimal("5000"))
        record = generate_130_page_record(d)
        field = record[142:159]
        assert field == "N0000000000400000"  # -4000.00

    def test_pago_20_pct_at_correct_position(self):
        d = make_130(ingresos=Decimal("10000"), gastos=Decimal("3000"))
        record = generate_130_page_record(d)
        # [04] at positions 160-176 (0-indexed: 159-175), unsigned
        field = record[159:176]
        assert field == "00000000000140000"  # 20% of 7000 = 1400.00

    def test_resultado_at_correct_position(self):
        d = make_130(ingresos=Decimal("10000"))
        record = generate_130_page_record(d)
        # [19] at positions 415-431 (0-indexed: 414-430), signed
        field = record[414:431]
        # resultado = 20% of 10000 = 2000
        assert field == " 0000000000200000"

    def test_iban_at_correct_position(self):
        d = make_130(
            ingresos=Decimal("10000"),
            cuenta_iban="ES60 0049 1500 0512 3456 7892",
        )
        record = generate_130_page_record(d)
        # IBAN at positions 446-479 (0-indexed: 445-478)
        iban_field = record[445:479]
        assert iban_field.startswith("ES600049150005123456789")

    def test_complementaria_flag(self):
        d = make_130(
            ingresos=Decimal("10000"),
            complementaria=True,
            justificante_anterior="1234567890123",
        )
        record = generate_130_page_record(d)
        # Complementaria at position 432 (0-indexed: 431)
        assert record[431] == "X"
        # Justificante at positions 433-445 (0-indexed: 432-444)
        assert record[432:445] == "1234567890123"

    def test_no_complementaria(self):
        d = make_130(ingresos=Decimal("10000"))
        record = generate_130_page_record(d)
        assert record[431] == " "

    def test_agricultural_fields_zeros(self):
        d = make_130()
        record = generate_130_page_record(d)
        # [08] pos 228-244 (0-indexed: 227-243) should be zeros
        assert record[227:244] == "00000000000000000"
        # [11] pos 279-295 (0-indexed: 278-294) signed zero
        assert record[278:295] == " 0000000000000000"


class TestGenerate130Wrapper:
    def test_contains_open_tag(self):
        d = make_130(exercise=2026, period="1T")
        wrapper = generate_130_wrapper(d)
        assert "<T130020261T0000>" in wrapper

    def test_contains_closing_tag(self):
        d = make_130(exercise=2026, period="1T")
        wrapper = generate_130_wrapper(d)
        assert "</T130020261T0000>" in wrapper

    def test_contains_aux(self):
        d = make_130()
        wrapper = generate_130_wrapper(d)
        assert "<AUX>" in wrapper
        assert "</AUX>" in wrapper

    def test_contains_page_record(self):
        d = make_130()
        wrapper = generate_130_wrapper(d)
        assert "<T13001000>" in wrapper
        assert "</T13001000>" in wrapper


class TestGenerate130Boe:
    def test_delegates_to_wrapper(self):
        d = make_130()
        boe = generate_130_boe(d)
        wrapper = generate_130_wrapper(d)
        assert boe == wrapper

    def test_single_line_no_newlines(self):
        d = make_130()
        boe = generate_130_boe(d)
        assert "\n" not in boe
        assert "\r" not in boe
        assert "\t" not in boe

    def test_encodable_as_iso_8859_1(self):
        d = make_130()
        boe = generate_130_boe(d)
        encoded = boe.encode("iso-8859-1")
        assert isinstance(encoded, bytes)

    def test_complete_structure(self):
        d = make_130(exercise=2026, period="2T")
        boe = generate_130_boe(d)
        assert boe.startswith("<T130020262T0000>")
        assert boe.endswith("</T130020262T0000>")
        assert "<AUX>" in boe
        assert "<T13001000>" in boe
        assert "</T13001000>" in boe

    def test_realistic_q1_scenario(self):
        """Typical Q1 IRPF advance for a freelancer."""
        d = make_130(
            nif="12345678Z",
            apellidos="GARCIA LOPEZ",
            nombre="JUAN",
            exercise=2026,
            period="1T",
            ingresos=Decimal("8000"),
            gastos=Decimal("2500"),
        )
        # rendimiento = 5500, pago = 20% = 1100
        assert d.rendimiento_neto == Decimal("5500")
        assert d.pago_20_pct == Decimal("1100.00")
        assert d.resultado == Decimal("1100.00")
        assert d.tipo_declaracion == "I"

        boe = generate_130_boe(d)
        assert "12345678Z" in boe
        assert "GARCIA LOPEZ" in boe

    def test_q2_with_prior_payments(self):
        """Q2 with accumulated income and Q1 payment deducted."""
        d = make_130(
            nif="12345678Z",
            apellidos="GARCIA LOPEZ",
            nombre="JUAN",
            exercise=2026,
            period="2T",
            ingresos=Decimal("16000"),  # Year-to-date
            gastos=Decimal("5000"),
            pagos_anteriores=Decimal("1100"),  # Q1 payment
        )
        # rendimiento = 11000, pago = 2200, minus 1100 prior = 1100
        assert d.resultado == Decimal("1100.00")
        assert d.tipo_declaracion == "I"

    def test_no_income_quarter(self):
        """No income → negative."""
        d = make_130(
            nif="12345678Z",
            apellidos="GARCIA LOPEZ",
            nombre="JUAN",
            exercise=2026,
            period="1T",
        )
        assert d.resultado == Decimal("0")
        assert d.tipo_declaracion == "N"

    def test_loss_quarter(self):
        """More expenses than income → negative."""
        d = make_130(
            nif="12345678Z",
            apellidos="GARCIA LOPEZ",
            nombre="JUAN",
            exercise=2026,
            period="1T",
            ingresos=Decimal("2000"),
            gastos=Decimal("5000"),
        )
        assert d.rendimiento_neto == Decimal("-3000")
        assert d.pago_20_pct == Decimal("0")
        assert d.resultado == Decimal("0")
        assert d.tipo_declaracion == "N"
