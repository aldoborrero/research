# aeat-autonomo

CLI tool for generating and submitting AEAT tax declarations (Modelos 303 and 130) for Spanish autónomos.

Generates BOE fixed-width files matching AEAT's official diseño de registro, then submits them via Presentación Directa (JSON over HTTPS with mTLS).

## Setup

```bash
# Enter devshell (requires Nix with flakes)
nix develop

# Install the package in editable mode
pip install -e '.[dev]'

# Create a config file
aeat init
# Edit aeat-config.json with your real NIF, certificate path, and Quipu credentials
```

### Configuration

`aeat-config.json`:

```json
{
  "declarant": {
    "nif": "12345678A",
    "apellidos": "GARCIA LOPEZ",
    "nombre": "JUAN"
  },
  "certificate": {
    "pfx_path": "./certificado.p12",
    "password": "YOUR_PASSWORD"
  },
  "quipu": {
    "api_key": "YOUR_QUIPU_APP_ID",
    "api_secret": "YOUR_QUIPU_APP_SECRET"
  },
  "iban": "ES00 0000 0000 0000 0000 0000",
  "testing": true
}
```

You need an FNMT digital certificate (`.p12`) for submission. Get one at [sede.fnmt.gob.es](https://www.sede.fnmt.gob.es/certificados/persona-fisica).

## Usage

### Fetch totals from Quipu

```bash
aeat quipu-totals --year 2026 --quarter 1T
```

### Generate BOE files

```bash
# Modelo 303 (IVA) — from Quipu data
aeat generate 303 --year 2026 --quarter 1T --from-quipu -o 303_1T_2026.boe

# Modelo 303 — manual input
aeat generate 303 --year 2026 --quarter 1T -o 303_1T_2026.boe

# Modelo 130 (IRPF advance) — from Quipu data (accumulates Q1..Qn)
aeat generate 130 --year 2026 --quarter 1T --from-quipu -o 130_1T_2026.boe

# Modelo 130 — with previous payments
aeat generate 130 --year 2026 --quarter 2T --from-quipu --prev-payments 500.00 -o 130_2T_2026.boe
```

### Validate (dry run)

Validates the BOE file against AEAT's test environment and optionally saves a PDF preview:

```bash
aeat submit 303 --year 2026 --quarter 1T --file 303_1T_2026.boe --dry-run --save-pdf preview.pdf
aeat submit 130 --year 2026 --quarter 1T --file 130_1T_2026.boe --dry-run
```

### Submit

```bash
# For tipo=Ingreso (payment due), you need an NRC from the bank
aeat submit 303 --year 2026 --quarter 1T --file 303_1T_2026.boe --nrc XXXXXXXXXXXXX

# For tipo=Negativa/Compensar, no NRC needed
aeat submit 303 --year 2026 --quarter 1T --file 303_1T_2026.boe
```

## AEAT Endpoints

### Presentación Directa (Modelos 303, 130, 390)

Protocol: JSON POST with mutual TLS (client certificate). **Not SOAP.**

| Service | Environment | URL |
|---------|-------------|-----|
| Submit (PresBasicaDos) | Production | `https://www1.agenciatributaria.gob.es/wlpl/PFTW-PICW/PresBasicaDos` |
| Submit (PresBasicaDos) | Testing | `https://prewww1.aeat.es/wlpl/PFTW-PICW/PresBasicaDos` |
| Validate (ServValiDos) | Testing only | `https://prewww2.aeat.es/wlpl/PFTW-PICW/ServValiDos` |
| Query (ConsultaExt) | Production | `https://www1.agenciatributaria.gob.es/wlpl/SCEJ-MANT/ConsultaExt` |
| Query (ConsultaExt) | Testing | `https://prewww1.aeat.es/wlpl/SCEJ-MANT/ConsultaExt` |

### TGVI Online (Declaraciones informativas: 347, 349)

Protocol: HTTP file upload with client certificate.

| Service | Environment | URL |
|---------|-------------|-----|
| Submit | Production | `https://www1.agenciatributaria.gob.es/wlpl/inwinvoc/es.aeat.dit.adi.eama.jdit.ws.DRServicioDeclaracionREST` |
| Submit | Testing | `https://prewww2.aeat.es/wlpl/inwinvoc/es.aeat.dit.adi.eama.jdit.ws.DRServicioDeclaracionREST` |

### Testing with the Pre-production Environment

The AEAT test environment (`prewww1`/`prewww2`) has important constraints:

- **Availability**: Weekdays only, approximately 8:00–15:00h Spanish time (CET/CEST)
- **Certificate**: Your real FNMT certificate works in pre. The NIF in `FIRNIF` must match the certificate.
- **Validation endpoint** (`ServValiDos`): Returns a PDF preview of the declaration without actually filing it. Only available on `prewww2`.
- **Submissions in pre are not real**: Declarations submitted to `prewww1` are not filed with Hacienda. Use this to test the full flow.
- **Set `"testing": true`** in your config (the default) to use pre-production endpoints.

Workflow for testing:

```bash
# 1. Generate the BOE file
aeat generate 303 --year 2026 --quarter 1T --from-quipu -o test.boe

# 2. Validate and get PDF preview (pre environment, weekdays 8-15h)
aeat submit 303 --year 2026 --quarter 1T --file test.boe --dry-run --save-pdf preview.pdf

# 3. Review preview.pdf — does it look right?

# 4. When ready for real submission, set "testing": false in config
aeat submit 303 --year 2026 --quarter 1T --file test.boe --nrc YOUR_NRC
```

## Architecture

```
src/aeat_autonomo/
├── cli.py          # Click CLI (generate, submit, quipu-totals, init)
├── config.py       # CertificateConfig, AEATEndpoints, QuipuConfig
├── modelo303.py    # Modelo 303 BOE generator (Pages 01 + 03)
├── modelo130.py    # Modelo 130 BOE generator (600-position record)
├── submit.py       # PresentacionDirectaClient + TGVIOnlineClient
├── quipu.py        # Quipu REST API client (OAuth2 + JSON:API)
└── boe.py          # Low-level BOE formatting helpers
```

### BOE File Format

Both models use AEAT's tagged fixed-width record format:

- **Modelo 303**: Multi-page structure — `<T30301000>` (IVA devengado + deducible) + `<T30303000>` (resultado + payment)
- **Modelo 130**: Single 600-position record — `<T13001000>` (IRPF pago fraccionado)

Numeric fields: sign char (`' '`/`N`) + zero-padded integer + 2-digit centimos. Example: `1234.56 €` → `" 000000000123456"` (17 chars).

### Quipu Integration

Uses Quipu's REST API (`https://getquipu.com/api`) with OAuth2 client credentials. Fetches issued invoices (income) and book entries (expenses) for the quarter, then aggregates base + VAT totals.

Reference implementation: [numtide/freelancer-toolbox/quipu](https://github.com/numtide/freelancer-toolbox/tree/main/quipu/quipu_api)

## References

- [AEAT Diseños de Registro (Modelos 300-399)](https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro/modelos-300-399.html)
- [AEAT Diseños de Registro (Modelos 100-199)](https://sede.agenciatributaria.gob.es/Sede/ayuda/disenos-registro/modelos-100-199.html)
- [AEAT Servicios Comunes — Presentación Directa](https://sede.agenciatributaria.gob.es/Sede/ayuda/consultas-informaticas/presentacion-declaraciones-ayuda-tecnica.html)
- [OCA/l10n-spain Modelo 303 (Odoo)](https://github.com/OCA/l10n-spain/tree/16.0/l10n_es_aeat_mod303)
- [Quipu API Docs](https://getquipu.com/developers)
