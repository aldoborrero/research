"""AEAT submission clients — Servicios Comunes SOAP and TGVI Online.

Servicios Comunes v2.7 is used for autoliquidaciones (303, 130, 390).
TGVI Online is used for informative declarations (347, 349).

Both use client certificate (mTLS) authentication via .p12/PFX files.
"""

from __future__ import annotations

import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

import httpx
from requests_pkcs12 import Pkcs12Adapter

from .config import CertificateConfig


@dataclass
class SubmissionResult:
    """Result of an AEAT declaration submission."""

    success: bool
    csv: str  # Código Seguro de Verificación — receipt code
    timestamp: str
    raw_response: str
    errors: list[str]


def _convert_p12_to_pem(pfx_path: Path, password: str) -> tuple[Path, Path]:
    """Convert a .p12 file to separate PEM cert and key files.

    Required for zeep/httpx which don't natively support PKCS#12.
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


class ServiciosComunesClient:
    """Client for AEAT Servicios Comunes Declaraciones v2.7.

    Used to submit autoliquidaciones (303, 130, 390) via SOAP.
    The service accepts the tagged BOE format (<T...>) directly.

    Production endpoint:
        https://www1.agenciatributaria.gob.es/wlpl/SSGD-TGVI/ws/ServicioDeclaracion

    Testing endpoint:
        https://prewww1.aeat.es/wlpl/SSGD-TGVI/ws/ServicioDeclaracion
    """

    PROD_URL = (
        "https://www1.agenciatributaria.gob.es"
        "/wlpl/SSII-FACT/ws/ServicioDeclaracion"
    )
    TEST_URL = (
        "https://prewww1.aeat.es"
        "/wlpl/SSII-FACT/ws/ServicioDeclaracion"
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

        # Convert p12 to PEM for httpx
        self._cert_pem, self._key_pem = _convert_p12_to_pem(
            cert_config.pfx_path, cert_config.password
        )

    def submit(self, modelo: str, boe_content: str) -> SubmissionResult:
        """Submit a declaration via Servicios Comunes.

        Args:
            modelo: Model number (e.g., "303", "130").
            boe_content: The tagged BOE content string.

        Returns:
            SubmissionResult with CSV receipt code on success.
        """
        # Build the SOAP envelope for presentación directa
        soap_body = self._build_soap_envelope(modelo, boe_content)

        with httpx.Client(
            cert=(str(self._cert_pem), str(self._key_pem)),
            verify=True,
            timeout=60.0,
        ) as client:
            response = client.post(
                self._base_url,
                content=soap_body.encode("utf-8"),
                headers={
                    "Content-Type": "text/xml; charset=utf-8",
                    "SOAPAction": '""',
                },
            )

        return self._parse_response(response.text)

    def validate(self, modelo: str, boe_content: str) -> SubmissionResult:
        """Validate a declaration without submitting.

        Same as submit but uses the validation endpoint.
        """
        # For validation, we change the SOAP action
        soap_body = self._build_soap_envelope(modelo, boe_content, action="validar")

        with httpx.Client(
            cert=(str(self._cert_pem), str(self._key_pem)),
            verify=True,
            timeout=60.0,
        ) as client:
            response = client.post(
                self._base_url,
                content=soap_body.encode("utf-8"),
                headers={
                    "Content-Type": "text/xml; charset=utf-8",
                    "SOAPAction": '"validar"',
                },
            )

        return self._parse_response(response.text)

    def _build_soap_envelope(
        self, modelo: str, boe_content: str, action: str = "presentar"
    ) -> str:
        """Build SOAP XML envelope for declaration submission."""
        # Escape XML special characters in BOE content
        escaped_boe = (
            boe_content.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
        )

        return f"""<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope
    xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
    xmlns:dec="https://www2.agenciatributaria.gob.es/static_files/common/internet/dep/tgvi/ws/DeclaracionType.xsd">
  <soapenv:Header/>
  <soapenv:Body>
    <dec:{action}Declaracion>
      <dec:modelo>{modelo}</dec:modelo>
      <dec:declaracion>{escaped_boe}</dec:declaracion>
    </dec:{action}Declaracion>
  </soapenv:Body>
</soapenv:Envelope>"""

    def _parse_response(self, xml_response: str) -> SubmissionResult:
        """Parse AEAT SOAP response into SubmissionResult."""
        # Simple XML parsing — in production use lxml/zeep response parsing
        import re

        csv_match = re.search(r"<csv>(.+?)</csv>", xml_response, re.IGNORECASE)
        timestamp_match = re.search(
            r"<timestamp>(.+?)</timestamp>", xml_response, re.IGNORECASE
        )
        error_matches = re.findall(
            r"<error>(.+?)</error>", xml_response, re.IGNORECASE
        )

        success = csv_match is not None and not error_matches

        return SubmissionResult(
            success=success,
            csv=csv_match.group(1) if csv_match else "",
            timestamp=timestamp_match.group(1) if timestamp_match else "",
            raw_response=xml_response,
            errors=error_matches,
        )

    def close(self) -> None:
        """Clean up temporary PEM files."""
        self._cert_pem.unlink(missing_ok=True)
        self._key_pem.unlink(missing_ok=True)

    def __enter__(self) -> ServiciosComunesClient:
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
        "https://prewww2.aeat.es"
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

        self._cert_pem, self._key_pem = _convert_p12_to_pem(
            cert_config.pfx_path, cert_config.password
        )

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

        # TGVI returns a simple response with CSV and errors
        return self._parse_response(response.text)

    def _parse_response(self, response_text: str) -> SubmissionResult:
        """Parse TGVI Online response."""
        import re

        csv_match = re.search(r"CSV:(\S+)", response_text)
        errors = re.findall(r"ERROR:(.+)", response_text)

        return SubmissionResult(
            success=csv_match is not None and not errors,
            csv=csv_match.group(1) if csv_match else "",
            timestamp="",
            raw_response=response_text,
            errors=errors,
        )

    def close(self) -> None:
        self._cert_pem.unlink(missing_ok=True)
        self._key_pem.unlink(missing_ok=True)

    def __enter__(self) -> TGVIOnlineClient:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()
