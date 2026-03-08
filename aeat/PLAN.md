# AEAT REST API — Implementation Plan

## Overview

Add a FastAPI-based REST API to `aeat-autonomo` that exposes the existing CLI functionality via HTTP endpoints, designed for integration with n8n and similar automation platforms.

---

## A. Framework Choice: FastAPI

**Why FastAPI is the best fit:**

| Criterion | FastAPI | Flask | Litestar |
|---|---|---|---|
| OpenAPI/Swagger auto-generation | Built-in (n8n can import OpenAPI specs) | Manual/extension | Built-in |
| Pydantic validation | Native | Manual | Native |
| Async support | Native (httpx is already async-ready) | Requires extensions | Native |
| Community/ecosystem | Largest modern Python API community | Largest overall | Small |
| n8n HTTP Request node | JSON in/out, auto-docs for testing | Works but no auto-docs | Works |
| Learning curve | Low (existing httpx/dataclass patterns match) | Low | Medium |

**Key advantage for n8n**: FastAPI auto-generates an OpenAPI 3.0 spec at `/openapi.json`. n8n's HTTP Request node can work with any JSON API, but more importantly, if you ever want to build a custom n8n node, the OpenAPI spec can be imported directly.

---

## B. API Design

### Endpoints

```
GET  /health                          → Health check + config status
GET  /api/v1/quipu/totals             → Fetch quarterly totals from Quipu
POST /api/v1/generate/303             → Generate Modelo 303 BOE file
POST /api/v1/generate/130             → Generate Modelo 130 BOE file
POST /api/v1/submit/303               → Submit Modelo 303 to AEAT
POST /api/v1/submit/130               → Submit Modelo 130 to AEAT
POST /api/v1/validate/{modelo}        → Validate declaration (dry-run)
GET  /api/v1/simulate                 → Simulate full fiscal year
```

### Request/Response Examples

**Generate 303:**
```json
// POST /api/v1/generate/303
{
  "year": 2026,
  "quarter": "1T",
  "from_quipu": true
}
// OR with manual data:
{
  "year": 2026,
  "quarter": "1T",
  "base_21": "1000.00",
  "cuota_21": "210.00",
  "base_deducible_interior": "500.00",
  "cuota_deducible_interior": "105.00"
}
```

```json
// Response 200
{
  "boe_content": "<T3030...>",
  "summary": {
    "iva_devengado": "210.00",
    "iva_deducible": "105.00",
    "resultado": "105.00",
    "resultado_liquidacion": "105.00",
    "tipo_declaracion": "I"
  }
}
```

**Submit 303:**
```json
// POST /api/v1/submit/303
{
  "year": 2026,
  "quarter": "1T",
  "boe_content": "<T3030...>",
  "nrc": "optional-payment-ref"
}
```

```json
// Response 200
{
  "success": true,
  "csv": "ABC1234567890123",
  "justificante": "1234567890123",
  "fecha": "20260308",
  "hora": "12:30:00",
  "pdf_url": "https://...",
  "warnings": []
}
```

**Quipu Totals:**
```json
// GET /api/v1/quipu/totals?year=2026&quarter=1T
// Response 200
{
  "year": 2026,
  "quarter": "1T",
  "total_income_gross": "5000.00",
  "total_vat_collected": "1050.00",
  "total_expenses_gross": "2000.00",
  "total_vat_deductible": "420.00",
  "net_vat": "630.00",
  "net_income": "3000.00"
}
```

### Error Responses

All errors follow a consistent JSON format (n8n-friendly):

```json
{
  "error": "validation_error",
  "detail": "Invalid quarter: must be one of 1T, 2T, 3T, 4T",
  "status_code": 422
}
```

---

## C. Authentication: API Key Header

- Callers send `X-API-Key: <secret>` header with every request
- API key configured via `AEAT_API_KEY` env var (or `api_key` in config file)
- FastAPI dependency checks the header and returns 401 if missing/invalid
- `/health` endpoint is unauthenticated (for monitoring)

**n8n setup**: In the HTTP Request node, add a header `X-API-Key` = `{{$credentials.aeatApiKey}}`, or use n8n's "Header Auth" credential type.

---

## D. Configuration: File + Environment Override

Priority order (highest wins):
1. Environment variables (`AEAT_CERT_PATH`, `AEAT_CERT_PASSWORD`, `AEAT_QUIPU_KEY`, etc.)
2. Config file (`aeat-config.json`, path configurable via `AEAT_CONFIG`)

This allows:
- Development: use the existing `aeat-config.json`
- Production/Docker: override secrets via env vars, keep structure in file
- n8n: server holds all config; API callers only send form-specific parameters

### Environment Variables

| Variable | Maps to | Default |
|---|---|---|
| `AEAT_CONFIG` | Config file path | `aeat-config.json` |
| `AEAT_API_KEY` | API authentication key | (required) |
| `AEAT_CERT_PATH` | `certificate.pfx_path` | from config file |
| `AEAT_CERT_PASSWORD` | `certificate.password` | from config file |
| `AEAT_QUIPU_KEY` | `quipu.api_key` | from config file |
| `AEAT_QUIPU_SECRET` | `quipu.api_secret` | from config file |
| `AEAT_TESTING` | `testing` flag | `true` |
| `AEAT_HOST` | API bind host | `0.0.0.0` |
| `AEAT_PORT` | API bind port | `8000` |

---

## E. Architecture

### New Files

```
src/aeat_autonomo/
├── api/
│   ├── __init__.py          # FastAPI app factory
│   ├── config.py            # Settings (Pydantic BaseSettings, file + env)
│   ├── auth.py              # API key dependency
│   ├── routes_generate.py   # /generate/303, /generate/130
│   ├── routes_submit.py     # /submit/303, /submit/130, /validate/{modelo}
│   ├── routes_quipu.py      # /quipu/totals
│   ├── routes_simulate.py   # /simulate
│   └── schemas.py           # Pydantic request/response models
```

### Entry Point

New CLI command + direct uvicorn entry:

```python
# CLI: aeat serve --host 0.0.0.0 --port 8000
# Direct: uvicorn aeat_autonomo.api:app --host 0.0.0.0 --port 8000
```

### Key Design Decisions

1. **Reuse existing modules directly** — The API routes call the same functions as the CLI (`generate_303_boe`, `PresentacionDirectaClient.submit`, etc.). No abstraction layer needed; the existing code is already well-separated.

2. **Certificate lifecycle** — The `PresentacionDirectaClient` converts `.p12` to PEM on init. For the API, create it once at startup and reuse across requests (with proper cleanup on shutdown).

3. **Sync endpoints are fine** — AEAT calls take <60s. FastAPI can run sync functions in a thread pool. No need for async complexity since httpx sync client is used.

4. **BOE content as string** — Generate endpoints return the BOE content as a JSON string field. Submit endpoints accept it the same way. This keeps the API JSON-only (no file uploads needed for n8n integration).

---

## F. n8n Integration Notes

### How n8n Will Use This

Typical n8n workflow:
1. **Schedule Trigger** → quarterly cron (e.g., April 1, July 1, Oct 1, Jan 1)
2. **HTTP Request** → `GET /api/v1/quipu/totals?year=2026&quarter=1T` (fetch accounting data)
3. **HTTP Request** → `POST /api/v1/generate/303` with `from_quipu: true` (generate form)
4. **HTTP Request** → `POST /api/v1/submit/303` with the BOE content from step 3
5. **Telegram/Email node** → notify with CSV receipt number

### n8n-Friendly Patterns

- **JSON everywhere** — no XML, no file uploads, no multipart
- **Flat error responses** — n8n's IF node can check `{{$json.error}}` easily
- **Query params for GET** — n8n handles these natively in the HTTP Request node
- **Consistent response shape** — every endpoint returns predictable JSON keys
- **OpenAPI spec** — available at `/openapi.json` for documentation and potential custom node generation

---

## G. Dependencies to Add

```toml
[project.optional-dependencies]
api = [
    "fastapi>=0.115",
    "uvicorn[standard]>=0.30",
    "pydantic-settings>=2.0",
]
```

This keeps the API as an optional dependency — the CLI continues to work without FastAPI installed.

---

## H. Implementation Steps

1. Add `fastapi`, `uvicorn`, `pydantic-settings` to `pyproject.toml` as optional deps
2. Create `src/aeat_autonomo/api/` package with:
   - `config.py` — Pydantic `BaseSettings` loading from file + env
   - `auth.py` — API key check dependency
   - `schemas.py` — Pydantic models for requests/responses
   - Route modules for each endpoint group
   - `__init__.py` — app factory with startup/shutdown hooks
3. Add `aeat serve` CLI command
4. Update `devshell.nix` to include API dependencies
5. Add tests using `pytest` + `httpx` (TestClient)
