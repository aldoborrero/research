"""Tests for quipu.py — Quipu API client and QuarterlyTotals."""

import re
from decimal import Decimal

import pytest

from aeat_autonomo.config import QuipuConfig
from aeat_autonomo.quipu import QuarterlyTotals, QuipuClient


# ---------------------------------------------------------------------------
# QuarterlyTotals computed properties
# ---------------------------------------------------------------------------


class TestQuarterlyTotals:
    def test_net_vat_positive(self):
        t = QuarterlyTotals(
            total_income_gross=Decimal("10000"),
            total_vat_collected=Decimal("2100"),
            total_expenses_gross=Decimal("5000"),
            total_vat_deductible=Decimal("1050"),
        )
        assert t.net_vat == Decimal("1050")

    def test_net_vat_negative(self):
        t = QuarterlyTotals(
            total_income_gross=Decimal("1000"),
            total_vat_collected=Decimal("210"),
            total_expenses_gross=Decimal("5000"),
            total_vat_deductible=Decimal("1050"),
        )
        assert t.net_vat == Decimal("-840")

    def test_net_vat_zero(self):
        t = QuarterlyTotals(
            total_income_gross=Decimal("5000"),
            total_vat_collected=Decimal("1050"),
            total_expenses_gross=Decimal("5000"),
            total_vat_deductible=Decimal("1050"),
        )
        assert t.net_vat == Decimal("0")

    def test_net_income_positive(self):
        t = QuarterlyTotals(
            total_income_gross=Decimal("10000"),
            total_vat_collected=Decimal("2100"),
            total_expenses_gross=Decimal("3000"),
            total_vat_deductible=Decimal("630"),
        )
        assert t.net_income == Decimal("7000")

    def test_net_income_negative(self):
        t = QuarterlyTotals(
            total_income_gross=Decimal("2000"),
            total_vat_collected=Decimal("420"),
            total_expenses_gross=Decimal("5000"),
            total_vat_deductible=Decimal("1050"),
        )
        assert t.net_income == Decimal("-3000")

    def test_net_income_zero(self):
        t = QuarterlyTotals(
            total_income_gross=Decimal("5000"),
            total_vat_collected=Decimal("1050"),
            total_expenses_gross=Decimal("5000"),
            total_vat_deductible=Decimal("1050"),
        )
        assert t.net_income == Decimal("0")


# ---------------------------------------------------------------------------
# QuipuClient — mocked HTTP tests
# ---------------------------------------------------------------------------


@pytest.fixture
def quipu_config():
    return QuipuConfig(
        api_key="test-key",
        api_secret="test-secret",
        base_url="https://getquipu.com/api",
    )


def _oauth_response():
    """Standard OAuth token response."""
    return {"access_token": "test-token-abc123", "token_type": "bearer"}


def _invoices_response(items: list[dict], has_next: bool = False) -> dict:
    """Build a Quipu JSON:API response for invoices/book_entries."""
    data = []
    for item in items:
        data.append({
            "id": str(item.get("id", 1)),
            "type": "invoices",
            "attributes": {
                "total_amount": str(item["total_amount"]),
                "vat_amount": str(item["vat_amount"]),
            },
        })
    links = {}
    if has_next:
        links["next"] = "https://getquipu.com/api/invoices?page[number]=2"
    return {"data": data, "links": links}


def _mock_quipu_calls(httpx_mock, invoices_json=None, book_entries_json=None):
    """Register OAuth + API mocks for a single get_quarterly_totals call."""
    httpx_mock.add_response(
        url="https://getquipu.com/oauth/token",
        json=_oauth_response(),
    )
    httpx_mock.add_response(
        url=re.compile(r".*/invoices.*"),
        json=invoices_json or _invoices_response([]),
    )
    httpx_mock.add_response(
        url=re.compile(r".*/book_entries.*"),
        json=book_entries_json or _invoices_response([]),
    )


class TestQuipuClientAuth:
    def test_authenticate_sends_credentials(self, quipu_config, httpx_mock):
        _mock_quipu_calls(httpx_mock)

        with QuipuClient(quipu_config) as client:
            client.get_quarterly_totals(2026, 1)

        # Verify OAuth request was made
        auth_request = httpx_mock.get_requests()[0]
        assert auth_request.url == "https://getquipu.com/oauth/token"
        body = auth_request.read().decode()
        assert "test-key" in body
        assert "test-secret" in body

    def test_token_is_cached(self, quipu_config, httpx_mock):
        _mock_quipu_calls(httpx_mock)

        with QuipuClient(quipu_config) as client:
            client.get_quarterly_totals(2026, 1)
            # Second call — add API mocks but no new OAuth mock
            httpx_mock.add_response(
                url=re.compile(r".*/invoices.*"),
                json=_invoices_response([]),
            )
            httpx_mock.add_response(
                url=re.compile(r".*/book_entries.*"),
                json=_invoices_response([]),
            )
            client.get_quarterly_totals(2026, 2)

        oauth_requests = [
            r for r in httpx_mock.get_requests()
            if "oauth" in str(r.url)
        ]
        assert len(oauth_requests) == 1


class TestQuipuClientQuarterlyTotals:
    def test_invalid_quarter_raises(self, quipu_config):
        with QuipuClient(quipu_config) as client:
            with pytest.raises(ValueError, match="Invalid quarter: 5"):
                client.get_quarterly_totals(2026, 5)

    def test_invalid_quarter_zero(self, quipu_config):
        with QuipuClient(quipu_config) as client:
            with pytest.raises(ValueError, match="Invalid quarter: 0"):
                client.get_quarterly_totals(2026, 0)

    def test_q1_date_range(self, quipu_config, httpx_mock):
        _mock_quipu_calls(httpx_mock)

        with QuipuClient(quipu_config) as client:
            client.get_quarterly_totals(2026, 1)

        invoice_req = [
            r for r in httpx_mock.get_requests()
            if "invoices" in str(r.url)
        ][0]
        assert "filter%5Bdate_from%5D=2026-01-01" in str(invoice_req.url)
        assert "filter%5Bdate_to%5D=2026-04-01" in str(invoice_req.url)

    def test_q4_date_range(self, quipu_config, httpx_mock):
        _mock_quipu_calls(httpx_mock)

        with QuipuClient(quipu_config) as client:
            client.get_quarterly_totals(2026, 4)

        invoice_req = [
            r for r in httpx_mock.get_requests()
            if "invoices" in str(r.url)
        ][0]
        assert "filter%5Bdate_from%5D=2026-10-01" in str(invoice_req.url)
        assert "filter%5Bdate_to%5D=2026-12-31" in str(invoice_req.url)

    def test_aggregates_income_and_expenses(self, quipu_config, httpx_mock):
        _mock_quipu_calls(
            httpx_mock,
            invoices_json=_invoices_response([
                {"id": 1, "total_amount": "5000.00", "vat_amount": "1050.00"},
                {"id": 2, "total_amount": "3000.00", "vat_amount": "630.00"},
            ]),
            book_entries_json=_invoices_response([
                {"id": 10, "total_amount": "2000.00", "vat_amount": "420.00"},
            ]),
        )

        with QuipuClient(quipu_config) as client:
            totals = client.get_quarterly_totals(2026, 1)

        assert totals.total_income_gross == Decimal("8000.00")
        assert totals.total_vat_collected == Decimal("1680.00")
        assert totals.total_expenses_gross == Decimal("2000.00")
        assert totals.total_vat_deductible == Decimal("420.00")
        assert totals.net_vat == Decimal("1260.00")
        assert totals.net_income == Decimal("6000.00")

    def test_empty_quarter(self, quipu_config, httpx_mock):
        _mock_quipu_calls(httpx_mock)

        with QuipuClient(quipu_config) as client:
            totals = client.get_quarterly_totals(2026, 1)

        assert totals.total_income_gross == Decimal("0")
        assert totals.total_vat_collected == Decimal("0")
        assert totals.total_expenses_gross == Decimal("0")
        assert totals.total_vat_deductible == Decimal("0")

    def test_pagination(self, quipu_config, httpx_mock):
        httpx_mock.add_response(
            url="https://getquipu.com/oauth/token",
            json=_oauth_response(),
        )
        # Page 1 of invoices (has next)
        httpx_mock.add_response(
            url=re.compile(r".*/invoices.*"),
            json=_invoices_response(
                [{"id": 1, "total_amount": "1000.00", "vat_amount": "210.00"}],
                has_next=True,
            ),
        )
        # Page 2 of invoices (no next)
        httpx_mock.add_response(
            url=re.compile(r".*/invoices.*"),
            json=_invoices_response(
                [{"id": 2, "total_amount": "2000.00", "vat_amount": "420.00"}],
                has_next=False,
            ),
        )
        # Expenses (single page)
        httpx_mock.add_response(
            url=re.compile(r".*/book_entries.*"),
            json=_invoices_response([]),
        )

        with QuipuClient(quipu_config) as client:
            totals = client.get_quarterly_totals(2026, 1)

        assert totals.total_income_gross == Decimal("3000.00")
        assert totals.total_vat_collected == Decimal("630.00")


class TestQuipuClientBearerAuth:
    def test_sends_bearer_token(self, quipu_config, httpx_mock):
        _mock_quipu_calls(httpx_mock)

        with QuipuClient(quipu_config) as client:
            client.get_quarterly_totals(2026, 1)

        api_requests = [
            r for r in httpx_mock.get_requests()
            if "oauth" not in str(r.url)
        ]
        for req in api_requests:
            assert req.headers["authorization"] == "Bearer test-token-abc123"
            assert req.headers["accept"] == "application/vnd.quipu.v1+json"


# ---------------------------------------------------------------------------
# Integration: QuarterlyTotals → Modelo data mapping
# ---------------------------------------------------------------------------


class TestQuipuToModeloMapping:
    """Verify that Quipu data maps correctly to form inputs."""

    def _sample_totals(self) -> QuarterlyTotals:
        return QuarterlyTotals(
            total_income_gross=Decimal("10000"),
            total_vat_collected=Decimal("2100"),
            total_expenses_gross=Decimal("4000"),
            total_vat_deductible=Decimal("840"),
        )

    def test_maps_to_modelo303(self):
        """QuarterlyTotals feeds directly into Modelo303Data fields."""
        from aeat_autonomo.modelo303 import Modelo303Data

        t = self._sample_totals()
        d = Modelo303Data(
            nif="12345678Z",
            nombre_razon="TEST USER",
            exercise=2026,
            period="1T",
            base_21=t.total_income_gross,
            cuota_21=t.total_vat_collected,
            base_deducible_interior=t.total_expenses_gross,
            cuota_deducible_interior=t.total_vat_deductible,
        )
        assert d.resultado_liquidacion == t.net_vat
        assert d.tipo_declaracion == "I"

    def test_maps_to_modelo130(self):
        """QuarterlyTotals feeds directly into Modelo130Data fields."""
        from aeat_autonomo.modelo130 import Modelo130Data

        t = self._sample_totals()
        d = Modelo130Data(
            nif="12345678Z",
            apellidos="TEST",
            nombre="USER",
            exercise=2026,
            period="1T",
            ingresos=t.total_income_gross,
            gastos=t.total_expenses_gross,
        )
        assert d.rendimiento_neto == t.net_income
        assert d.tipo_declaracion == "I"
