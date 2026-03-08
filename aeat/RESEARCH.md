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

**Cons:** Nix packaging requires workarounds for browser binaries, heavy
dependency footprint.

**Known client certificate issues (as of 2025-2026):**

- Using `channel: 'chrome'` (system Chrome) instead of bundled Chromium can
  cause certificate selection popups or "No client certificate provided" errors
  ([#33230](https://github.com/microsoft/playwright/issues/33230))
- Certificates signed by private CAs may fail with SSL handshake errors; the
  `ca` argument is not exposed
  ([#33414](https://github.com/microsoft/playwright/issues/33414))
- Some Keycloak/CBA flows fail to show certificate dialog
  ([#33563](https://github.com/microsoft/playwright/issues/33563))
- Playwright implements client certs via a SOCKS proxy interceptor, which
  explains some edge cases with non-bundled browsers
- **Recommendation:** Use Playwright's bundled Chromium (`browserName:
  'chromium'` without `channel`) and set `ignore_https_errors=True` for best
  client certificate support

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

## 5. Target Models — Per-Model Analysis

### 5.1 Modelo 347 — Declaración anual de operaciones con terceras personas

- **What:** Annual declaration of operations with third parties exceeding
  €3,005.06
- **Frequency:** Annual (February)
- **Submission method:** **TGVI Online** (machine-to-machine, confirmed)
- **File format:** BOE/ASCII plain text, ISO-8859-1 encoding
- **Diseño de registro:** [Modelos 300-399](https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro/modelos-300-399.html)
  — "347 - Ejercicio 2025 y siguientes" (PDF, 332 KB)
- **Record types:** Type 1 (header, one per declaration) + Type 2 (one per
  declared party/property)
- **Automation path:** `requests-pkcs12` → TGVI Online HTTP upload
- **Complexity:** Medium — file generation is the main task

### 5.2 Modelo 349 — Declaración recapitulativa de operaciones intracomunitarias

- **What:** Recapitulative declaration of intra-community operations
- **Frequency:** Monthly or Quarterly (depending on volume)
- **Deadline:** Within 20 days after the period ends
- **Submission method:** **TGVI Online** (machine-to-machine, confirmed)
- **File format:** BOE/ASCII plain text, ISO-8859-1 encoding
- **Diseño de registro:** [Modelos 300-399](https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro/modelos-300-399.html)
  — "349 - Orden HAC/174/2020" (PDF, 894 KB)
- **Automation path:** `requests-pkcs12` → TGVI Online HTTP upload
- **Complexity:** Medium — similar to 347

### 5.3 Modelo 303 — IVA Autoliquidación trimestral

- **What:** Quarterly VAT self-assessment
- **Frequency:** Quarterly (1-20 Apr/Jul/Oct, 1-30 Jan for Q4)
- **Submission method:** **Web form only** (sede electrónica) — no SOAP API
- **File format:** BOE format for import into the web form (`.303` extension).
  Can be generated externally and imported via "Importar" button
- **Diseño de registro:** [Modelos 300-399](https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro/modelos-300-399.html)
  — "303 - Ejercicio 2025" (XLSX, 380 KB)
- **Pre303Ayuda:** AEAT's help service can pre-fill fields from SII data or
  imported libro registro
- **Automation path:** Generate BOE file → Playwright imports into web form →
  submit. Also supports "presentación por lotes"
- **API coming:** New XML/WSDL-based API expected **January 2027** (Orden
  HAC/747/2025)
- **Complexity:** High — requires browser automation for submission

### 5.4 Modelo 390 — Resumen anual IVA

- **What:** Annual VAT summary declaration
- **Frequency:** Annual (1-30 January)
- **Submission method:** **Web form with file import** (sede electrónica)
- **File format:** BOE format (`.390` extension), can import via "Leer
  declaración"
- **Diseño de registro:** [Modelos 300-399](https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro/modelos-300-399.html)
  — "390 - Ejercicio 2025" (XLSX, 544 KB)
- **Automation path:** Generate BOE file → Playwright imports into web form →
  submit
- **Complexity:** High — similar to 303 (browser automation needed)

### 5.5 Modelo 130 — Pago fraccionado IRPF (estimación directa)

- **What:** Quarterly IRPF advance payment for self-employed (direct estimation)
- **Frequency:** Quarterly (same deadlines as 303)
- **Submission method:** **Web form (Pre130)** with file import support and
  **presentación por lotes**
- **File format:** BOE format (`.130` extension), plain text
- **Diseño de registro:** [Modelos 100-199](https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro.html)
- **Pre130 service:** Can auto-fill from AEAT's imported libro registro data
- **Automation path:** Generate BOE file → Playwright imports into web form →
  submit. Batch ("por lotes") presentation may allow multiple 130s at once
- **Complexity:** Medium-High — browser automation but simpler form than 303

### 5.6 Modelo 100 — Declaración anual IRPF (Renta)

- **What:** Annual personal income tax return
- **Frequency:** Annual (April-June)
- **Submission method:** **Renta WEB** (interactive platform) or **file upload
  in XML format** via "Presentación mediante fichero generado con programa de
  ayuda"
- **File format:** **XML** (not BOE) — must follow published XML schema
- **Diseño de registro:** [Modelos 100-199](https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro.html)
- **Submission URL:** [Presentación mediante fichero](https://sede.agenciatributaria.gob.es/Sede/ayuda/consultas-informaticas/renta-ayuda-tecnica/presentar-declaracion-mediante-fichero-generado-externo.html)
- **Automation path:** Generate XML file → upload via sede electrónica web form
  → confirm and sign
- **Testing:** Available at `preportal.aeat.es` under Renta section
- **Complexity:** Very High — most complex form, XML format, Renta WEB is a
  full SPA

### 5.7 Summary Matrix

| Model | Type          | Freq     | API?          | File Format | Automation Method        |
| ----- | ------------- | -------- | ------------- | ----------- | ------------------------ |
| 347   | Informative   | Annual   | **TGVI Online** | BOE/ASCII | HTTP upload (no browser) |
| 349   | Informative   | Q/M      | **TGVI Online** | BOE/ASCII | HTTP upload (no browser) |
| 303   | Self-assess   | Quarterly| No (2027)     | BOE         | Browser (Playwright)     |
| 390   | Summary       | Annual   | No            | BOE         | Browser (Playwright)     |
| 130   | Self-assess   | Quarterly| No            | BOE         | Browser (Playwright)     |
| 100   | Self-assess   | Annual   | No            | **XML**     | Browser (Playwright)     |

## 6. Recommended Implementation Plan

### Phase 1: Foundation

1. Set up Nix development environment with Python, `requests-pkcs12`, `zeep`,
   and Playwright
2. Implement BOE file generator library (shared across 347, 349, 303, 390, 130)
3. Parse diseño de registro specs for each model
4. **Prerequisite:** FNMT `.p12` certificate

### Phase 2: TGVI Online Models (347, 349) — Easiest wins

1. Implement TGVI Online HTTP client with certificate auth
2. Build file generators for 347 and 349 BOE formats
3. Test against `prewww2.aeat.es` / TGVI Online test endpoints
4. These two models can be fully automated without a browser

### Phase 3: Browser-Automated Self-Assessments (303, 390, 130)

1. Set up Playwright with client certificate for sede electrónica
2. Build BOE file generators for 303, 390, and 130
3. Automate: generate file → import into web form → verify → submit
4. Handle the "presentación por lotes" path for 130 if beneficial
5. Test against `preportal.aeat.es`

### Phase 4: Renta / Modelo 100

1. Build XML generator following AEAT's published schema
2. Automate file upload via sede electrónica
3. Handle Renta WEB's SPA flow if direct file upload is insufficient
4. This is the most complex model — tackle last

### Known Blockers

- **FNMT certificate required** — needed for all submission paths
- **Models 303, 390, 130, 100 have no API** — browser automation required
- **Nix + Playwright browser binaries** — requires workarounds for packaging
- **Diseño de registro specs** — need to download and parse for each model

## 7. Open Questions

Before proceeding with implementation:

1. Do you have an FNMT `.p12` certificate available? If so, is it a personal
   certificate or a company representative certificate?
2. Do you have access to the AEAT testing environment (`preportal.aeat.es`)?
3. Is there accounting data that needs to be ingested, or will form data be
   provided manually?
4. For Modelo 100 (Renta) — do you use Renta WEB's borrador, or do you generate
   the full declaration externally?
5. Priority order — should we start with the quarterly models (303, 130) since
   they recur most often, or with the TGVI models (347, 349) since they're
   easiest to automate?
