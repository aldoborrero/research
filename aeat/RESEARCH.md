# AEAT Automation — Research Summary

> Date: 2026-03-08

## 1. Authentication Methods

### 1.1 Certificate-Based Login (.p12 / .pfx) — RECOMMENDED

AEAT's sede electronica uses **mutual TLS (mTLS)** for authentication. The
client presents a valid X.509 certificate during the TLS handshake. There is no
browser-specific challenge flow, CAPTCHA, or JavaScript-dependent auth step for
the certificate path.

**Accepted certificates:**

- FNMT (Fábrica Nacional de Moneda y Timbre) personal certificates — most
  common
- DNIe (electronic national ID card) certificates
- Other qualified certificates from recognized Spanish CAs

FNMT certificates are issued as `.p12` / `.pfx` files (PKCS#12 format),
bundling private key + public certificate + CA chain, protected by a password.

**Programmatic usage — fully viable:**

```python
# Option A: requests-pkcs12 (simplest, uses .p12 directly)
from requests_pkcs12 import get
response = get(
    'https://www1.agenciatributaria.gob.es/...',
    pkcs12_filename='certificado.p12',
    pkcs12_password='password'
)

# Option B: Convert .p12 to PEM, use standard requests
import requests
response = requests.get(url, cert=('cert.crt', 'cert.key'))
```

```bash
# Option C: curl
curl --cert-type P12 --cert certificado.p12:"password" https://...
```

**Caveats:**

- FNMT `.p12` files must be converted to PEM for Zeep (SOAP client).
- ISO-8859-1 encoding is required for certain file-based submissions.
- For SOAP web services (SII, VeriFactu), XML messages are NOT individually
  signed — authentication is purely at the TLS layer.

### 1.2 DNIe / Cl@ve — NOT VIABLE FOR AUTOMATION

| Method           | Programmatic? | Why                                          |
| ---------------- | ------------- | -------------------------------------------- |
| DNIe             | No            | Requires physical smart card reader + PKCS#11 middleware |
| Cl@ve PIN        | No            | Interactive browser flow + SMS OTP           |
| Cl@ve Permanente | No            | Interactive browser flow + SMS OTP           |
| Cl@ve Firma      | No            | Requires interactive consent via app/SMS     |

**Conclusion: Certificate-based auth (.p12/.pfx) is the only realistic
programmatic approach.**

### 1.3 Session/Cookie Reuse

Not necessary. Since certificate auth works natively with `requests` and `curl`,
there is no practical reason to manually authenticate and reuse cookies. AEAT's
primary auth is at the TLS layer, not cookie-based.

## 2. AEAT Official APIs & Web Services

### 2.1 SII (Suministro Inmediato de Información del IVA)

The most mature AEAT API. SOAP 1.1 web service for near-real-time submission of
VAT invoice records.

- **Auth:** Client certificate (mTLS)
- **Protocol:** HTTPS + SOAP 1.1
- **Production:** `www1.agenciatributaria.gob.es` / `www2.agenciatributaria.gob.es`
- **Testing:** `prewww1.aeat.es` / `prewww2.aeat.es`
- **Services:** Issued invoices, received invoices, capital goods,
  intra-community operations, and more

### 2.2 VeriFactu (Mandatory from 2026)

Certified billing/invoicing record submission system.

- **Auth:** Client certificate (mTLS)
- **Protocol:** HTTPS + SOAP 1.1
- **Max records per submission:** 1,000
- **Testing:** `prewww10.aeat.es`
- **Deadlines:** Jul 2025 (software manufacturers), Jan 2026 (companies), Jul
  2026 (self-employed)

### 2.3 TGVI Online (Informative Declarations — Machine-to-Machine)

For models 190, 347, 349, and many others. Supports programmatic file upload
with certificate auth.

- **File format:** Plain text (ASCII/BOE format), ISO-8859-1
- **Supported models:** 038, 156, 159, 170, 180, 181, 182, 184, 185, 187, 188,
  189, 190, 192, 193, 194, 195, 196, 198, 199, 270, 280, 291, 296, 345, 346,
  347, 349, 611, 616, 720, 901, 943, and more

### 2.4 Modelo 303 (IVA Quarterly Self-Assessment)

Currently **no dedicated API**. Options today:

- Sede electronica web form (with certificate auth)
- File upload in BOE/ASCII format via the web form
- **New XML/WSDL-based API coming January 2027** (per Orden HAC/747/2025)

### 2.5 NIF Validation

Simple SOAP service for validating Spanish tax IDs (NIF/CIF).

### 2.6 Developer Portal

Official: `https://www.agenciatributaria.es/AEAT.desarrolladores/`

Publishes technical specs, XML schemas, WSDLs, FAQs, and provides testing
environment access at `preportal.aeat.es`.

## 3. Automation Framework Evaluation

### 3.1 Comparison Matrix

| Tool              | .p12 Native  | JS Support | Nix Ready | Maturity  | Verdict           |
| ----------------- | ------------ | ---------- | --------- | --------- | ----------------- |
| **Playwright**    | Yes (built-in) | Yes      | Partial   | High      | **Best choice**   |
| Puppeteer         | No           | Yes        | Partial   | High      | Not recommended   |
| Selenium          | No (profile) | Yes        | Good      | Very High | Viable fallback   |
| requests + pkcs12 | Yes (via lib)| No         | Good      | High      | For API-only flows|
| curl / httpx      | Yes          | No         | Excellent | Very High | For testing/APIs  |
| AI tools (Skyvern)| No           | Yes        | No        | Medium    | If forms complex  |

### 3.2 Playwright (Recommended for browser automation)

Native `.p12` support since v1.46 via `client_certificates`:

```python
context = browser.new_context(
    client_certificates=[{
        "origin": "https://sede.agenciatributaria.gob.es",
        "pfxPath": "./certificate.p12",
        "passphrase": "secret",
    }],
    ignore_https_errors=True,
)
```

**Pros:** Native .p12 support, multi-browser, auto-waiting, sync+async Python
APIs, active development.

**Cons:** Nix packaging requires workarounds for browser binaries, some reported
client certificate bugs, heavy dependency footprint.

### 3.3 Selenium (Fallback)

No direct API for client certs. Requires importing `.p12` into a Firefox profile
via `certutil`/`pk12util` and setting `security.default_personal_cert` to
`"Select Automatically"`. Better Nix packaging but more operational complexity.

### 3.4 Python requests + requests-pkcs12 (For direct API calls)

Simplest approach for non-browser HTTP interactions. Perfect for SOAP endpoints
(SII, VeriFactu) and TGVI Online submissions. Lightest dependency footprint.

## 4. Existing Libraries & Prior Art

### Python

| Library | Description |
| ------- | ----------- |
| [gisce/sii](https://github.com/gisce/sii) | Standalone SII library |
| [initios/aeat-web-services](https://github.com/initios/aeat-web-services) | AEAT web service integration (customs) |
| [raquinber/pyAEAT](https://github.com/raquinber/pyAEAT) | Generates data for AEAT models |
| [hokus15/ArrendaToolsModelo303](https://github.com/hokus15/ArrendaToolsModelo303) | Generates import strings for Modelo 303 |
| OCA/l10n-spain (l10n_es_aeat modules) | Odoo modules for full AEAT compliance |
| [requests-pkcs12](https://pypi.org/project/requests-pkcs12/) | .p12 support for Python requests |

### Go

| Library | Description |
| ------- | ----------- |
| [invopop/gobl.verifactu](https://github.com/invopop/gobl.verifactu) | GOBL invoices → VeriFactu XML |

### Other

| Library | Language | Description |
| ------- | -------- | ----------- |
| [mdiago/VeriFactu](https://github.com/mdiago/VeriFactu) | C# | VeriFactu implementation (309 stars) |
| [josemmo/Verifactu-PHP](https://github.com/josemmo/Verifactu-PHP) | PHP | PHP VeriFactu implementation |
| [fawno/AEAT](https://github.com/fawno/AEAT) | PHP | PHP classes for AEAT web services |

## 5. Recommended Implementation Plan

### Phase 1: Certificate Auth + Direct API Access

1. Set up Nix development environment with Python, `requests-pkcs12`, and `zeep`
2. Verify certificate authentication works against AEAT test environment
3. Implement SII and/or TGVI Online submission via SOAP/HTTP
4. **Prerequisite:** User must have an FNMT `.p12` certificate available

### Phase 2: Browser Automation (if needed)

1. Set up Playwright with Nix (using `playwright-driver.browsers`)
2. Configure client certificate for sede electronica
3. Automate Modelo 303 submission via the web form
4. Handle form navigation, field population, and submission confirmation

### Phase 3: Full Pipeline

1. Data ingestion from accounting sources
2. Form generation + automated submission
3. Confirmation retrieval and archival

### Known Blockers

- **FNMT certificate required** — must clarify which certificate the user has
- **Modelo 303 has no API until Jan 2027** — browser automation needed for now
- **Nix + Playwright browser binaries** — requires workarounds for packaging
- **Testing environment access** — need to verify test endpoints accept
  certificate

## 6. Open Questions

Before proceeding with implementation:

1. Do you have an FNMT `.p12` certificate available? If so, is it a personal
   certificate or a company representative certificate?
2. Which specific forms/models do you need to automate first? (e.g., Modelo 303,
   SII, informative declarations)
3. Do you have access to the AEAT testing environment (`preportal.aeat.es`)?
4. Is there accounting data that needs to be ingested, or will form data be
   provided manually?
