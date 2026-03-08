"""AEAT submission clients — Presentación Directa (JSON API) and TGVI Online.

CRITICAL: Servicios Comunes v27.1 is NOT a SOAP service. It is a plain
JSON-over-HTTPS API using mutual TLS (client certificate) authentication.

Endpoints:
  - Presentación Directa: POST JSON → PresBasicaDos
  - Validación + PDF: POST JSON → ServValiDos (test env only)
  - Consulta: POST form-urlencoded → ConsultaExt

Supported models: 303, 130, 390, 100, 111, 115, 200, 202, 210, and many more.
NOT supported: 006, 568, and all Declaraciones Informativas (use TGVI Online).

Reference: AEAT Servicios Comunes Declaraciones v27.1 specification.
"""

from __future__ import annotations

import base64
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import httpx

from .config import CertificateConfig


@dataclass
class SubmissionResult:
    """Result of an AEAT declaration submission."""

    success: bool
    csv: str = ""  # Código Seguro de Verificación (16 chars)
    justificante: str = ""  # Receipt number (13 chars)
    fecha: str = ""  # Date of submission
    hora: str = ""  # Time of submission
    pdf_url: str = ""  # URL to download signed PDF
    pdf_base64: str = ""  # Base64 PDF (from validation)
    warnings: list[str] = field(default_factory=list)
    errors: list[str] = field(default_factory=list)
    raw_response: dict[str, Any] | str = field(default_factory=dict)


def _convert_p12_to_pem(pfx_path: Path, password: str) -> tuple[Path, Path]:
    """Convert a .p12 file to separate PEM cert and key files.

    Returns (cert_path, key_path) as temporary files.
    """
    cert_file = tempfile.NamedTemporaryFile(suffix=".pem", delete=False)
    key_file = tempfile.NamedTemporaryFile(suffix=".pem", delete=False)

    # Extract certificate
    subprocess.run(
        [
            "openssl", "pkcs12", "-in", str(pfx_path),
            "-clcerts", "-nokeys", "-out", cert_file.name,
            "-passin", f"pass:{password}",
        ],
        check=True,
        capture_output=True,
    )

    # Extract private key
    subprocess.run(
        [
            "openssl", "pkcs12", "-in", str(pfx_path),
            "-nocerts", "-nodes", "-out", key_file.name,
            "-passin", f"pass:{password}",
        ],
        check=True,
        capture_output=True,
    )

    return Path(cert_file.name), Path(key_file.name)


def _strip_boe(content: str) -> str:
    """Strip CRLF and tabs from BOE content for the F01 JSON field.

    AEAT requires the flat file as a single unbroken string — no newlines
    or tabs allowed in the F01 field.
    """
    return content.replace("\r", "").replace("\n", "").replace("\t", "")


class PresentacionDirectaClient:
    """Client for AEAT Presentación Directa (autoliquidaciones).

    Submits declarations for models 303, 130, 390, etc. via JSON POST
    with mutual TLS authentication.

    Protocol: JSON over HTTPS (NOT SOAP).
    Auth: Client certificate — NIF in FIRNIF must match the certificate.

    Endpoints:
        Production: https://www1.agenciatributaria.gob.es/wlpl/PFTW-PICW/PresBasicaDos
        Testing:    https://prewww1.aeat.es/wlpl/PFTW-PICW/PresBasicaDos
    """

    PROD_PRESENTACION = (
        "https://www1.agenciatributaria.gob.es/wlpl/PFTW-PICW/PresBasicaDos"
    )
    TEST_PRESENTACION = (
        "https://prewww1.aeat.es/wlpl/PFTW-PICW/PresBasicaDos"
    )

    PROD_CONSULTA = (
        "https://www1.agenciatributaria.gob.es/wlpl/SCEJ-MANT/ConsultaExt"
    )
    TEST_CONSULTA = (
        "https://prewww1.aeat.es/wlpl/SCEJ-MANT/ConsultaExt"
    )

    # Validation is only available in the test environment
    TEST_VALIDACION = (
        "https://prewww2.aeat.es/wlpl/PFTW-PICW/ServValiDos"
    )

    def __init__(
        self,
        cert_config: CertificateConfig,
        *,
        nif_presentador: str,
        nombre_presentador: str,
        testing: bool = True,
    ) -> None:
        self._cert_config = cert_config
        self._nif = nif_presentador
        self._nombre = nombre_presentador
        self._testing = testing
        self._cert_pem: Path | None = None
        self._key_pem: Path | None = None

        # Convert p12 to PEM for httpx
        try:
            self._cert_pem, self._key_pem = _convert_p12_to_pem(
                cert_config.pfx_path, cert_config.password
            )
        except Exception:
            self.close()
            raise

    def _client(self) -> httpx.Client:
        return httpx.Client(
            cert=(str(self._cert_pem), str(self._key_pem)),
            verify=True,
            timeout=60.0,
        )

    def submit(
        self,
        modelo: str,
        ejercicio: str,
        periodo: str,
        boe_content: str,
        nrc: str = "",
    ) -> SubmissionResult:
        """Submit a declaration via Presentación Directa.

        Args:
            modelo: Model number (e.g., "303", "130").
            ejercicio: Tax year (e.g., "2026").
            periodo: Period (e.g., "1T", "2T", "3T", "4T").
            boe_content: The BOE flat file content.
            nrc: Número de Referencia Completo — required only for
                 payment type "I" (Ingreso). Leave empty for N/D/C/U.

        Returns:
            SubmissionResult with CSV receipt code on success.
        """
        payload = {
            "MODELO": modelo,
            "EJERCICIO": ejercicio,
            "PERIODO": periodo,
            "NRC": nrc,
            "IDI": "ES",
            "F01": _strip_boe(boe_content),
            "FIR": "FirmaBasica",
            "FIRNIF": self._nif,
            "FIRNOMBRE": self._nombre,
        }

        url = self.TEST_PRESENTACION if self._testing else self.PROD_PRESENTACION

        with self._client() as client:
            response = client.post(
                url,
                json=payload,
                headers={"Content-Type": "application/json;charset=UTF-8"},
            )
            response.raise_for_status()

        return self._parse_presentacion_response(response.json())

    def validate(
        self,
        modelo: str,
        ejercicio: str,
        periodo: str,
        boe_content: str,
    ) -> SubmissionResult:
        """Validate a declaration and get a PDF preview.

        Only available on the test environment (prewww2.aeat.es).
        Weekdays 8:00-15:00h Spanish time.

        Returns:
            SubmissionResult with pdf_base64 on success.
        """
        payload = {
            "MODELO": modelo,
            "EJERCICIO": ejercicio,
            "PERIODO": periodo,
            "IDI": "ES",
            "F01": _strip_boe(boe_content),
        }

        with self._client() as client:
            response = client.post(
                self.TEST_VALIDACION,
                json=payload,
                headers={"Content-Type": "application/json;charset=UTF-8"},
            )
            response.raise_for_status()

        return self._parse_validacion_response(response.json())

    def consultar(
        self,
        modelo: str,
        ejercicio: str,
        periodo: str = "",
        fecha_desde: str = "",
        fecha_hasta: str = "",
    ) -> str:
        """Query previously submitted declarations.

        Args:
            fecha_desde/fecha_hasta: Format YYYYMMDD.

        Returns:
            Raw XML response conforming to AEAT's consultas.xsd.
        """
        data = {
            "NIF": self._nif,
            "ANR": self._nombre,
            "MOD": modelo,
            "EJF": ejercicio,
            "PER": periodo,
            "FED": fecha_desde,
            "FEH": fecha_hasta,
            "HOD": "",
            "HOH": "",
        }

        url = self.TEST_CONSULTA if self._testing else self.PROD_CONSULTA

        with self._client() as client:
            response = client.post(
                url,
                data=data,
                headers={"Content-Type": "application/x-www-form-urlencoded"},
            )
            response.raise_for_status()

        return response.text

    def _parse_presentacion_response(self, data: dict[str, Any]) -> SubmissionResult:
        """Parse JSON response from PresBasicaDos."""
        respuesta = data.get("respuesta", {})

        if "correcta" in respuesta:
            ok = respuesta["correcta"]
            return SubmissionResult(
                success=True,
                csv=ok.get("CodigoSeguroVerificacion", ""),
                justificante=ok.get("Justificante", ""),
                fecha=ok.get("Fecha", ""),
                hora=ok.get("Hora", ""),
                pdf_url=ok.get("urlPdf", ""),
                warnings=ok.get("avisos", []) + ok.get("advertencias", []),
                raw_response=data,
            )

        if "errores" in respuesta:
            return SubmissionResult(
                success=False,
                errors=respuesta["errores"],
                raw_response=data,
            )

        return SubmissionResult(success=False, raw_response=data)

    def _parse_validacion_response(self, data: dict[str, Any]) -> SubmissionResult:
        """Parse JSON response from ServValiDos."""
        if "PDF" in data:
            return SubmissionResult(
                success=True,
                pdf_base64=data["PDF"],
                warnings=data.get("AVISOS", []),
                raw_response=data,
            )

        if "errores" in data.get("respuesta", {}):
            return SubmissionResult(
                success=False,
                errors=data["respuesta"]["errores"],
                raw_response=data,
            )

        return SubmissionResult(success=False, raw_response=data)

    def save_validation_pdf(self, result: SubmissionResult, path: Path) -> None:
        """Save the PDF from a validation result to a file."""
        if not result.pdf_base64:
            raise ValueError("No PDF in validation result")
        path.write_bytes(base64.b64decode(result.pdf_base64))

    def close(self) -> None:
        """Clean up temporary PEM files."""
        if self._cert_pem is not None:
            self._cert_pem.unlink(missing_ok=True)
        if self._key_pem is not None:
            self._key_pem.unlink(missing_ok=True)

    def __enter__(self) -> PresentacionDirectaClient:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()


class TGVIOnlineClient:
    """Client for AEAT TGVI Online — informative declarations (347, 349).

    Uses HTTP file upload with client certificate authentication.
    Accepts BOE/ASCII fixed-width files encoded in ISO-8859-1.
    """

    PROD_URL = (
        "https://www1.agenciatributaria.gob.es"
        "/wlpl/inwinvoc/es.aeat.dit.adi.eama.jdit.ws.DRServicioDeclaracionREST"
    )
    TEST_URL = (
        "https://prewww1.aeat.es"
        "/wlpl/inwinvoc/es.aeat.dit.adi.eama.jdit.ws.DRServicioDeclaracionREST"
    )

    def __init__(
        self,
        cert_config: CertificateConfig,
        *,
        testing: bool = True,
    ) -> None:
        self._cert_config = cert_config
        self._testing = testing
        self._base_url = self.TEST_URL if testing else self.PROD_URL
        self._cert_pem: Path | None = None
        self._key_pem: Path | None = None

        try:
            self._cert_pem, self._key_pem = _convert_p12_to_pem(
                cert_config.pfx_path, cert_config.password
            )
        except Exception:
            self.close()
            raise

    def submit(self, modelo: str, boe_bytes: bytes) -> SubmissionResult:
        """Submit an informative declaration via TGVI Online.

        Args:
            modelo: Model number (e.g., "347", "349").
            boe_bytes: The BOE file content as ISO-8859-1 encoded bytes.
        """
        with httpx.Client(
            cert=(str(self._cert_pem), str(self._key_pem)),
            verify=True,
            timeout=120.0,
        ) as client:
            response = client.post(
                f"{self._base_url}/presentar",
                content=boe_bytes,
                headers={
                    "Content-Type": "application/octet-stream",
                    "X-Modelo": modelo,
                },
            )

        return self._parse_response(response.text)

    def _parse_response(self, response_text: str) -> SubmissionResult:
        """Parse TGVI Online response."""
        import re

        csv_match = re.search(r"CSV:(\S+)", response_text)
        errors = re.findall(r"ERROR:(.+)", response_text)

        return SubmissionResult(
            success=csv_match is not None and not errors,
            csv=csv_match.group(1) if csv_match else "",
            raw_response=response_text,
            errors=errors,
        )

    def close(self) -> None:
        if self._cert_pem is not None:
            self._cert_pem.unlink(missing_ok=True)
        if self._key_pem is not None:
            self._key_pem.unlink(missing_ok=True)

    def __enter__(self) -> TGVIOnlineClient:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()
