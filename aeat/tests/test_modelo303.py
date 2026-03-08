"""Tests for modelo303.py — Modelo 303 IVA quarterly declaration."""

from decimal import Decimal

import pytest

from aeat_autonomo.modelo303 import (
    Modelo303Data,
    _an,
    _bool_yn,
    _cents,
    _num,
    _pct,
    _signed,
    generate_303_boe,
    _generate_page01,
    _generate_page03,
    _generate_wrapper,
)


# ---------------------------------------------------------------------------
# Helper function tests
# ---------------------------------------------------------------------------


class TestCents:
    def test_whole_euros(self):
        assert _cents(Decimal("100")) == 10000

    def test_with_centimos(self):
        assert _cents(Decimal("123.45")) == 12345

    def test_zero(self):
        assert _cents(Decimal("0")) == 0

    def test_rounding_half_up(self):
        assert _cents(Decimal("1.005")) == 101  # rounds up
        assert _cents(Decimal("1.004")) == 100

    def test_negative(self):
        assert _cents(Decimal("-50.25")) == -5025


class TestNum:
    def test_basic(self):
        assert _num(12345) == "00000000000012345"

    def test_zero(self):
        assert _num(0) == "00000000000000000"

    def test_negative_clamped_to_zero(self):
        assert _num(-100) == "00000000000000000"

    def test_custom_length(self):
        assert _num(42, 5) == "00042"

    def test_length_is_correct(self):
        assert len(_num(0)) == 17
        assert len(_num(999, 5)) == 5


class TestSigned:
    def test_positive(self):
        result = _signed(12345)
        assert result == " 0000000000012345"
        assert result[0] == " "

    def test_zero(self):
        result = _signed(0)
        assert result == " 0000000000000000"

    def test_negative(self):
        result = _signed(-12345)
        assert result == "N0000000000012345"
        assert result[0] == "N"

    def test_length(self):
        assert len(_signed(0)) == 17
        assert len(_signed(-999)) == 17
        assert len(_signed(999)) == 17

    def test_custom_length(self):
        result = _signed(-42, 5)
        assert result == "N0042"
        assert len(result) == 5


class TestPct:
    def test_21_percent(self):
        assert _pct(Decimal("21")) == "02100"

    def test_10_percent(self):
        assert _pct(Decimal("10")) == "01000"

    def test_4_percent(self):
        assert _pct(Decimal("4")) == "00400"

    def test_half_percent(self):
        assert _pct(Decimal("0.5")) == "00050"

    def test_zero(self):
        assert _pct(Decimal("0")) == "00000"

    def test_100_percent(self):
        assert _pct(Decimal("100")) == "10000"

    def test_length(self):
        assert len(_pct(Decimal("21"))) == 5


class TestAn:
    def test_padding(self):
        assert _an("ABC", 10) == "ABC       "

    def test_truncation(self):
        assert _an("ABCDEFGHIJ", 5) == "ABCDE"

    def test_uppercase(self):
        assert _an("hello", 5) == "HELLO"

    def test_exact_length(self):
        assert _an("ABCDE", 5) == "ABCDE"


class TestBoolYn:
    def test_true(self):
        assert _bool_yn(True) == "1"

    def test_false(self):
        assert _bool_yn(False) == "2"


# ---------------------------------------------------------------------------
# Modelo303Data computed properties
# ---------------------------------------------------------------------------


def make_303(**kwargs) -> Modelo303Data:
    """Create Modelo303Data with sensible defaults."""
    defaults = dict(
        nif="12345678Z",
        nombre_razon="GARCIA LOPEZ JUAN",
        exercise=2026,
        period="1T",
    )
    defaults.update(kwargs)
    return Modelo303Data(**defaults)


class TestModelo303DataComputedProperties:
    def test_total_cuota_devengada(self):
        d = make_303(
            cuota_4=Decimal("10"),
            cuota_10=Decimal("20"),
            cuota_21=Decimal("30"),
        )
        assert d.total_cuota_devengada == Decimal("60")

    def test_total_cuota_devengada_all_sources(self):
        d = make_303(
            cuota_4=Decimal("1"),
            cuota_10=Decimal("2"),
            cuota_21=Decimal("3"),
            cuota_intracom=Decimal("4"),
            cuota_inv_suj_pasivo=Decimal("5"),
            cuota_modificacion=Decimal("6"),
        )
        assert d.total_cuota_devengada == Decimal("21")

    def test_total_a_deducir(self):
        d = make_303(
            cuota_deducible_interior=Decimal("100"),
            cuota_deducible_inversion=Decimal("50"),
        )
        assert d.total_a_deducir == Decimal("150")

    def test_total_a_deducir_all_sources(self):
        d = make_303(
            cuota_deducible_interior=Decimal("1"),
            cuota_deducible_inversion=Decimal("2"),
            cuota_deducible_import_corr=Decimal("3"),
            cuota_deducible_import_inv=Decimal("4"),
            cuota_deducible_intracom_corr=Decimal("5"),
            cuota_deducible_intracom_inv=Decimal("6"),
            cuota_rectificacion=Decimal("7"),
            cuota_compensaciones=Decimal("8"),
            cuota_reg_inversiones=Decimal("9"),
            cuota_reg_prorrata=Decimal("10"),
        )
        assert d.total_a_deducir == Decimal("55")

    def test_resultado_regimen_general(self):
        d = make_303(
            cuota_21=Decimal("210"),
            cuota_deducible_interior=Decimal("100"),
        )
        assert d.resultado_regimen_general == Decimal("110")

    def test_resultado_regimen_general_negative(self):
        d = make_303(
            cuota_21=Decimal("100"),
            cuota_deducible_interior=Decimal("200"),
        )
        assert d.resultado_regimen_general == Decimal("-100")

    def test_suma_resultados_equals_regimen_general(self):
        d = make_303(cuota_21=Decimal("100"))
        assert d.suma_resultados == d.resultado_regimen_general

    def test_atribuible_estado_equals_suma(self):
        d = make_303(cuota_21=Decimal("100"))
        assert d.atribuible_estado == d.suma_resultados

    def test_resultado_with_compensacion(self):
        d = make_303(
            cuota_21=Decimal("500"),
            compensacion_anterior=Decimal("100"),
        )
        # resultado = atribuible_estado - compensacion + regularizacion
        assert d.resultado == Decimal("400")

    def test_resultado_with_regularizacion(self):
        d = make_303(
            cuota_21=Decimal("500"),
            regularizacion_anual=Decimal("50"),
        )
        assert d.resultado == Decimal("550")

    def test_resultado_liquidacion(self):
        d = make_303(
            cuota_21=Decimal("500"),
            deducir_periodos_ant=Decimal("100"),
        )
        assert d.resultado_liquidacion == Decimal("400")

    def test_full_chain(self):
        """Test the complete casilla chain with realistic values."""
        d = make_303(
            base_21=Decimal("10000"),
            cuota_21=Decimal("2100"),
            base_deducible_interior=Decimal("5000"),
            cuota_deducible_interior=Decimal("1050"),
            compensacion_anterior=Decimal("200"),
        )
        assert d.total_cuota_devengada == Decimal("2100")
        assert d.total_a_deducir == Decimal("1050")
        assert d.resultado_regimen_general == Decimal("1050")
        assert d.resultado == Decimal("850")  # 1050 - 200
        assert d.resultado_liquidacion == Decimal("850")


class TestModelo303TipoDeclaracion:
    def test_ingreso(self):
        d = make_303(cuota_21=Decimal("100"))
        assert d.tipo_declaracion == "I"

    def test_negativa_zero(self):
        d = make_303()
        assert d.tipo_declaracion == "N"

    def test_compensar_q1(self):
        d = make_303(
            period="1T",
            cuota_deducible_interior=Decimal("100"),
        )
        assert d.tipo_declaracion == "C"

    def test_compensar_q2(self):
        d = make_303(
            period="2T",
            cuota_deducible_interior=Decimal("100"),
        )
        assert d.tipo_declaracion == "C"

    def test_devolucion_q4(self):
        d = make_303(
            period="4T",
            cuota_deducible_interior=Decimal("100"),
        )
        assert d.tipo_declaracion == "D"

    def test_sin_actividad(self):
        d = make_303(sin_actividad=True)
        assert d.tipo_declaracion == "N"


# ---------------------------------------------------------------------------
# BOE generation tests
# ---------------------------------------------------------------------------


class TestGeneratePage01:
    def test_starts_with_tag(self):
        d = make_303()
        page = _generate_page01(d)
        assert page.startswith("<T30301000>")

    def test_ends_with_tag(self):
        d = make_303()
        page = _generate_page01(d)
        assert page.endswith("</T30301000>")

    def test_contains_nif(self):
        d = make_303(nif="12345678Z")
        page = _generate_page01(d)
        assert "12345678Z" in page

    def test_contains_name(self):
        d = make_303(nombre_razon="GARCIA LOPEZ JUAN")
        page = _generate_page01(d)
        assert "GARCIA LOPEZ JUAN" in page

    def test_contains_exercise(self):
        d = make_303(exercise=2026)
        page = _generate_page01(d)
        assert "2026" in page

    def test_contains_period(self):
        d = make_303(period="3T")
        page = _generate_page01(d)
        assert "3T" in page

    def test_tipo_declaracion_in_output(self):
        d = make_303(cuota_21=Decimal("100"))
        page = _generate_page01(d)
        # After header "<T30301000>" (11 chars) + " " (1 blank) = tipo at position 12
        after_header = page[len("<T30301000>"):]
        # First char is complementary indicator (blank), second is tipo
        assert after_header[1] == "I"

    def test_vat_rates_present(self):
        d = make_303(
            base_21=Decimal("1000"),
            cuota_21=Decimal("210"),
        )
        page = _generate_page01(d)
        # 21% rate should appear as "02100"
        assert "02100" in page


class TestGeneratePage03:
    def test_starts_with_tag(self):
        d = make_303()
        page = _generate_page03(d)
        assert page.startswith("<T30303000>")

    def test_ends_with_tag(self):
        d = make_303()
        page = _generate_page03(d)
        assert page.endswith("</T30303000>")

    def test_100_percent_atribuible(self):
        d = make_303()
        page = _generate_page03(d)
        # 100% = "10000"
        assert "10000" in page

    def test_complementaria_flag(self):
        d = make_303(complementaria=True, justificante_anterior="1234567890123")
        page = _generate_page03(d)
        assert "X" in page
        assert "1234567890123" in page

    def test_sin_actividad_flag(self):
        d = make_303(sin_actividad=True)
        page = _generate_page03(d)
        # sin_actividad "X" should be present after justificante field
        content = page[len("<T30303000>"):]
        # Find the sin_actividad marker — it's in the page03 output
        assert "X" in content

    def test_iban_included(self):
        d = make_303(cuenta_iban="ES60 0049 1500 0512 3456 7892")
        page = _generate_page03(d)
        assert "ES6000491500051234567892" in page


class TestGenerateWrapper:
    def test_contains_open_tag(self):
        d = make_303(exercise=2026, period="1T")
        wrapper = _generate_wrapper(d)
        assert "<T303020261T0000>" in wrapper

    def test_contains_aux(self):
        d = make_303()
        wrapper = _generate_wrapper(d)
        assert "<AUX>" in wrapper
        assert "</AUX>" in wrapper


class TestGenerate303Boe:
    def test_complete_structure(self):
        d = make_303(exercise=2026, period="2T")
        boe = generate_303_boe(d)
        assert boe.startswith("<T303020262T0000>")
        assert boe.endswith("</T303020262T0000>")
        assert "<AUX>" in boe
        assert "<T30301000>" in boe
        assert "</T30301000>" in boe
        assert "<T30303000>" in boe
        assert "</T30303000>" in boe

    def test_single_line_no_newlines(self):
        d = make_303()
        boe = generate_303_boe(d)
        assert "\n" not in boe
        assert "\r" not in boe
        assert "\t" not in boe

    def test_encodable_as_iso_8859_1(self):
        d = make_303()
        boe = generate_303_boe(d)
        encoded = boe.encode("iso-8859-1")
        assert isinstance(encoded, bytes)

    def test_realistic_autonomo_scenario(self):
        """Simulate a typical quarterly VAT declaration for a freelancer."""
        d = make_303(
            nif="12345678Z",
            nombre_razon="GARCIA LOPEZ JUAN",
            exercise=2026,
            period="1T",
            # Issued invoices: €5000 base at 21%
            base_21=Decimal("5000"),
            cuota_21=Decimal("1050"),
            # Deductible expenses: €2000 at 21%
            base_deducible_interior=Decimal("2000"),
            cuota_deducible_interior=Decimal("420"),
        )
        assert d.tipo_declaracion == "I"
        assert d.resultado_liquidacion == Decimal("630")

        boe = generate_303_boe(d)
        assert boe.startswith("<T303020261T0000>")
        assert "12345678Z" in boe
        # Check the result amount appears (63000 cents = €630.00)
        assert "0000000000063000" in boe

    def test_negative_result_compensar(self):
        """Quarter with more deductions than income → compensar."""
        d = make_303(
            nif="12345678Z",
            nombre_razon="GARCIA LOPEZ JUAN",
            exercise=2026,
            period="2T",
            base_21=Decimal("1000"),
            cuota_21=Decimal("210"),
            base_deducible_interior=Decimal("3000"),
            cuota_deducible_interior=Decimal("630"),
        )
        assert d.tipo_declaracion == "C"
        assert d.resultado_liquidacion == Decimal("-420")

        boe = generate_303_boe(d)
        # Negative amount should use N prefix
        assert "N0000000000042000" in boe

    def test_zero_activity(self):
        """No activity quarter."""
        d = make_303(
            nif="12345678Z",
            nombre_razon="GARCIA LOPEZ JUAN",
            exercise=2026,
            period="3T",
            sin_actividad=True,
        )
        assert d.tipo_declaracion == "N"
        boe = generate_303_boe(d)
        assert len(boe) > 0
