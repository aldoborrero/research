"""Tests for the simulate CLI command."""

import json
import re
from decimal import Decimal
from pathlib import Path

import pytest
from click.testing import CliRunner

from aeat_autonomo.cli import main


@pytest.fixture
def config_file(tmp_path):
    """Create a temporary config file."""
    config = {
        "declarant": {
            "nif": "12345678Z",
            "apellidos": "GARCIA LOPEZ",
            "nombre": "JUAN",
        },
        "quipu": {
            "api_key": "test-key",
            "api_secret": "test-secret",
        },
        "iban": "ES60 0049 1500 0512 3456 7892",
        "testing": True,
    }
    config_path = tmp_path / "aeat-config.json"
    config_path.write_text(json.dumps(config))
    return str(config_path)


def _oauth_response():
    return {"access_token": "test-token", "token_type": "bearer"}


def _quarter_response(income: str, vat_in: str, expenses: str, vat_out: str):
    """Build Quipu response for a single quarter."""
    data = []
    if Decimal(income) > 0 or Decimal(vat_in) > 0:
        data.append({
            "id": "1",
            "type": "invoices",
            "attributes": {"total_amount": income, "vat_amount": vat_in},
        })
    return {"data": data, "links": {}}


def _expenses_response(expenses: str, vat_out: str):
    data = []
    if Decimal(expenses) > 0 or Decimal(vat_out) > 0:
        data.append({
            "id": "1",
            "type": "book_entries",
            "attributes": {"total_amount": expenses, "vat_amount": vat_out},
        })
    return {"data": data, "links": {}}


def _mock_full_year(httpx_mock, quarterly_data):
    """Mock Quipu API for a full year of quarterly data.

    quarterly_data: list of 4 tuples: (income, vat_collected, expenses, vat_deductible)
    """
    httpx_mock.add_response(
        url="https://getquipu.com/oauth/token",
        json=_oauth_response(),
    )
    for income, vat_in, expenses, vat_out in quarterly_data:
        httpx_mock.add_response(
            url=re.compile(r".*/invoices.*"),
            json=_quarter_response(income, vat_in, expenses, vat_out),
        )
        httpx_mock.add_response(
            url=re.compile(r".*/book_entries.*"),
            json=_expenses_response(expenses, vat_out),
        )


class TestSimulateCommand:
    def test_full_year_output(self, config_file, httpx_mock):
        _mock_full_year(httpx_mock, [
            ("5000", "1050", "2000", "420"),
            ("6000", "1260", "2500", "525"),
            ("4000", "840", "1500", "315"),
            ("7000", "1470", "3000", "630"),
        ])

        runner = CliRunner()
        result = runner.invoke(main, ["--config", config_file, "simulate", "--year", "2024"])

        assert result.exit_code == 0
        # Check all quarters appear
        assert "2024 1T" in result.output
        assert "2024 2T" in result.output
        assert "2024 3T" in result.output
        assert "2024 4T" in result.output
        # Check annual summary
        assert "ANNUAL SUMMARY" in result.output

    def test_shows_quipu_data(self, config_file, httpx_mock):
        _mock_full_year(httpx_mock, [
            ("5000", "1050", "2000", "420"),
            ("0", "0", "0", "0"),
            ("0", "0", "0", "0"),
            ("0", "0", "0", "0"),
        ])

        runner = CliRunner()
        result = runner.invoke(main, ["--config", config_file, "simulate", "--year", "2024"])

        assert result.exit_code == 0
        assert "5,000.00" in result.output or "5000.00" in result.output
        assert "1,050.00" in result.output or "1050.00" in result.output

    def test_shows_modelo_303_results(self, config_file, httpx_mock):
        _mock_full_year(httpx_mock, [
            ("10000", "2100", "4000", "840"),
            ("0", "0", "0", "0"),
            ("0", "0", "0", "0"),
            ("0", "0", "0", "0"),
        ])

        runner = CliRunner()
        result = runner.invoke(main, ["--config", config_file, "simulate", "--year", "2024"])

        assert result.exit_code == 0
        assert "Modelo 303" in result.output
        # VAT result: 2100 - 840 = 1260
        assert "1260.00" in result.output

    def test_shows_modelo_130_results(self, config_file, httpx_mock):
        _mock_full_year(httpx_mock, [
            ("10000", "2100", "4000", "840"),
            ("0", "0", "0", "0"),
            ("0", "0", "0", "0"),
            ("0", "0", "0", "0"),
        ])

        runner = CliRunner()
        result = runner.invoke(main, ["--config", config_file, "simulate", "--year", "2024"])

        assert result.exit_code == 0
        assert "Modelo 130" in result.output
        # Net income: 10000 - 4000 = 6000, 20% = 1200
        assert "1200.00" in result.output

    def test_130_accumulates_across_quarters(self, config_file, httpx_mock):
        _mock_full_year(httpx_mock, [
            ("10000", "2100", "4000", "840"),
            ("8000", "1680", "3000", "630"),
            ("0", "0", "0", "0"),
            ("0", "0", "0", "0"),
        ])

        runner = CliRunner()
        result = runner.invoke(main, ["--config", config_file, "simulate", "--year", "2024"])

        assert result.exit_code == 0
        # Q2 should show accumulated income: 10000 + 8000 = 18000
        assert "18000.00" in result.output

    def test_subset_of_quarters(self, config_file, httpx_mock):
        # Only simulate Q1 and Q2
        httpx_mock.add_response(
            url="https://getquipu.com/oauth/token",
            json=_oauth_response(),
        )
        # Need Q1 and Q2 data
        for _ in range(2):
            httpx_mock.add_response(
                url=re.compile(r".*/invoices.*"),
                json=_quarter_response("5000", "1050", "2000", "420"),
            )
            httpx_mock.add_response(
                url=re.compile(r".*/book_entries.*"),
                json=_expenses_response("2000", "420"),
            )

        runner = CliRunner()
        result = runner.invoke(main, [
            "--config", config_file, "simulate",
            "--year", "2024", "--quarters", "1T,2T",
        ])

        assert result.exit_code == 0
        assert "2024 1T" in result.output
        assert "2024 2T" in result.output
        assert "2024 3T" not in result.output
        assert "2024 4T" not in result.output

    def test_saves_boe_files(self, config_file, httpx_mock, tmp_path):
        # Only Q1 for simplicity
        httpx_mock.add_response(
            url="https://getquipu.com/oauth/token",
            json=_oauth_response(),
        )
        httpx_mock.add_response(
            url=re.compile(r".*/invoices.*"),
            json=_quarter_response("5000", "1050", "2000", "420"),
        )
        httpx_mock.add_response(
            url=re.compile(r".*/book_entries.*"),
            json=_expenses_response("2000", "420"),
        )

        out_dir = tmp_path / "simulation"
        runner = CliRunner()
        result = runner.invoke(main, [
            "--config", config_file, "simulate",
            "--year", "2024", "--quarters", "1T",
            "-o", str(out_dir),
        ])

        assert result.exit_code == 0
        assert (out_dir / "modelo303_2024_1T.boe").exists()
        assert (out_dir / "modelo130_2024_1T.boe").exists()

        # BOE files should be valid ISO-8859-1 (single line, starts with tag)
        boe_303 = (out_dir / "modelo303_2024_1T.boe").read_bytes().decode("iso-8859-1")
        assert boe_303.startswith("<T3030")
        assert "\n" not in boe_303

        boe_130 = (out_dir / "modelo130_2024_1T.boe").read_bytes().decode("iso-8859-1")
        assert boe_130.startswith("<T1300")
        assert "\n" not in boe_130

    def test_annual_summary_totals(self, config_file, httpx_mock):
        _mock_full_year(httpx_mock, [
            ("5000", "1050", "2000", "420"),
            ("5000", "1050", "2000", "420"),
            ("5000", "1050", "2000", "420"),
            ("5000", "1050", "2000", "420"),
        ])

        runner = CliRunner()
        result = runner.invoke(main, ["--config", config_file, "simulate", "--year", "2024"])

        assert result.exit_code == 0
        # Total income: 4 * 5000 = 20000
        assert "20000.00" in result.output
        # Total expenses: 4 * 2000 = 8000
        assert "8000.00" in result.output
        # Net VAT: 4 * (1050-420) = 2520
        assert "2520.00" in result.output

    def test_invalid_quarter_exits(self, config_file):
        runner = CliRunner()
        result = runner.invoke(main, [
            "--config", config_file, "simulate",
            "--year", "2024", "--quarters", "5T",
        ])
        assert result.exit_code != 0
        assert "Invalid quarter" in result.output

    def test_no_income_year(self, config_file, httpx_mock):
        _mock_full_year(httpx_mock, [
            ("0", "0", "1000", "210"),
            ("0", "0", "500", "105"),
            ("0", "0", "0", "0"),
            ("0", "0", "0", "0"),
        ])

        runner = CliRunner()
        result = runner.invoke(main, ["--config", config_file, "simulate", "--year", "2024"])

        assert result.exit_code == 0
        # 130 should show N (negativa) for all quarters
        assert result.output.count("Tipo:                 N") >= 2

    def test_q4_devolucion(self, config_file, httpx_mock):
        # Expenses every quarter → negative VAT result each quarter
        _mock_full_year(httpx_mock, [
            ("0", "0", "2000", "420"),
            ("0", "0", "2000", "420"),
            ("0", "0", "2000", "420"),
            ("0", "0", "2000", "420"),
        ])

        runner = CliRunner()
        result = runner.invoke(main, ["--config", config_file, "simulate", "--year", "2024"])

        assert result.exit_code == 0
        # Q1-Q3 with negative VAT → "C" (compensar)
        assert "Tipo:                 C" in result.output
        # Q4 with negative VAT → "D" (devolución)
        assert "Tipo:                 D" in result.output
