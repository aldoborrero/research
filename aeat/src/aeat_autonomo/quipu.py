"""Quipu API client for extracting accounting data.

Quipu uses a JSON:API-style REST API with OAuth2 bearer tokens.
See: https://getquipu.com/developers
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal
from typing import Any

import httpx

from .config import QuipuConfig


@dataclass
class QuarterlyTotals:
    """Aggregated totals for a quarter, used by models 303 and 130."""

    # Income
    total_income_gross: Decimal  # Base imponible ingresos
    total_vat_collected: Decimal  # IVA repercutido

    # Expenses
    total_expenses_gross: Decimal  # Base imponible gastos
    total_vat_deductible: Decimal  # IVA soportado deducible

    # Derived
    @property
    def net_vat(self) -> Decimal:
        """VAT to pay (positive) or reclaim (negative)."""
        return self.total_vat_collected - self.total_vat_deductible

    @property
    def net_income(self) -> Decimal:
        """Net income (ingresos - gastos) for IRPF calculation."""
        return self.total_income_gross - self.total_expenses_gross


class QuipuClient:
    """Client for the Quipu REST API."""

    def __init__(self, config: QuipuConfig) -> None:
        self._config = config
        self._token: str | None = None
        self._http = httpx.Client(
            base_url=config.base_url,
            timeout=30.0,
        )

    def _authenticate(self) -> str:
        """Obtain OAuth2 bearer token from Quipu."""
        if self._token:
            return self._token

        resp = self._http.post(
            "https://getquipu.com/oauth/token",
            json={
                "grant_type": "client_credentials",
                "client_id": self._config.api_key,
                "client_secret": self._config.api_secret,
            },
        )
        resp.raise_for_status()
        self._token = resp.json()["access_token"]
        return self._token

    def _get(self, path: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        """Authenticated GET request."""
        token = self._authenticate()
        resp = self._http.get(
            path,
            params=params,
            headers={
                "Authorization": f"Bearer {token}",
                "Accept": "application/vnd.quipu.v1+json",
            },
        )
        resp.raise_for_status()
        result: dict[str, Any] = resp.json()
        return result

    def get_quarterly_totals(self, year: int, quarter: int) -> QuarterlyTotals:
        """Extract quarterly totals from Quipu invoices.

        Args:
            year: Fiscal year (e.g., 2026).
            quarter: Quarter number (1-4).

        Returns:
            Aggregated income/expense/VAT totals for the quarter.
        """
        if quarter not in (1, 2, 3, 4):
            raise ValueError(f"Invalid quarter: {quarter}")

        # Quarter date ranges (inclusive start, exclusive end)
        month_start = (quarter - 1) * 3 + 1
        month_end = quarter * 3
        date_from = f"{year}-{month_start:02d}-01"
        if month_end == 12:
            date_to = f"{year + 1}-01-01"
        else:
            date_to = f"{year}-{month_end + 1:02d}-01"

        # Fetch issued invoices (income)
        income = self._fetch_invoices("invoices", date_from, date_to)
        # Fetch received invoices (expenses)
        expenses = self._fetch_invoices("book_entries", date_from, date_to)

        return QuarterlyTotals(
            total_income_gross=income["base"],
            total_vat_collected=income["vat"],
            total_expenses_gross=expenses["base"],
            total_vat_deductible=expenses["vat"],
        )

    def _fetch_invoices(
        self, endpoint: str, date_from: str, date_to: str
    ) -> dict[str, Decimal]:
        """Fetch and aggregate invoices from a Quipu endpoint."""
        total_base = Decimal("0")
        total_vat = Decimal("0")
        page = 1

        while True:
            data = self._get(
                f"/{endpoint}",
                params={
                    "filter[date_from]": date_from,
                    "filter[date_to]": date_to,
                    "page[number]": page,
                    "page[size]": 100,
                },
            )

            for item in data.get("data", []):
                attrs = item.get("attributes", {})
                total_base += Decimal(str(attrs.get("total_amount", "0")))
                # Quipu stores VAT breakdown in line items; use retention fields
                total_vat += Decimal(str(attrs.get("vat_amount", "0")))

            # Check for next page
            links = data.get("links", {})
            if not links.get("next"):
                break
            page += 1

        return {"base": total_base, "vat": total_vat}

    def close(self) -> None:
        self._http.close()

    def __enter__(self) -> QuipuClient:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()
