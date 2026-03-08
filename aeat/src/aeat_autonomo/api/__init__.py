"""FastAPI application for AEAT tax automation.

Usage:
    uvicorn aeat_autonomo.api:app --host 0.0.0.0 --port 8000

Or via CLI:
    aeat serve --host 0.0.0.0 --port 8000
"""

from __future__ import annotations

from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
from decimal import InvalidOperation

from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse

from ..logging import get_logger, setup_logging
from ..clients.aeat import PresentacionDirectaClient
from .auth import configure_auth
from .config import ApiSettings, load_settings
from .routes_generate import router as generate_router
from .routes_quipu import router as quipu_router
from .routes_simulate import router as simulate_router
from .routes_submit import router as submit_router
from .schemas import HealthResponse

log = get_logger(__name__)

# Module-level settings and shared client, populated at startup
_settings: ApiSettings = ApiSettings()
_submit_client: PresentacionDirectaClient | None = None


@asynccontextmanager
async def _lifespan(application: FastAPI) -> AsyncGenerator[None]:
    """Manage application lifecycle — certificate init and cleanup."""
    global _submit_client  # noqa: PLW0603

    if _settings.certificate.is_configured:
        try:
            _submit_client = PresentacionDirectaClient(
                _settings.certificate,
                nif_presentador=_settings.declarant.nif,
                nombre_presentador=_settings.declarant.nombre_completo,
                testing=_settings.testing,
            )
            log.info(
                "submit_client_ready",
                nif=_settings.declarant.nif,
                testing=_settings.testing,
            )
        except Exception:
            log.error(
                "submit_client_init_failed",
                cert_path=str(_settings.certificate.pfx_path),
            )
            _submit_client = None

    log.info("api_started", host=_settings.host, port=_settings.port)
    yield

    # Cleanup
    if _submit_client is not None:
        _submit_client.close()
        log.info("submit_client_closed")


def create_app(settings: ApiSettings | None = None) -> FastAPI:
    """Create and configure the FastAPI application."""
    global _settings  # noqa: PLW0603

    # Configure structured logging (JSON for API)
    setup_logging(json_output=True)

    if settings is None:
        settings = load_settings()
    _settings = settings

    if settings.api_key:
        configure_auth(settings.api_key)

    log.info(
        "api_config_loaded",
        declarant_nif=settings.declarant.nif,
        certificate_configured=settings.certificate.is_configured,
        quipu_configured=settings.quipu.is_configured,
        testing=settings.testing,
    )

    application = FastAPI(
        title="AEAT Autonomo API",
        description=(
            "REST API for Spanish AEAT tax form generation and submission. "
            "Designed for integration with n8n and other automation platforms."
        ),
        version="0.1.0",
        lifespan=_lifespan,
    )

    # Health check (no auth required)
    @application.get("/health", response_model=HealthResponse, tags=["health"])
    def health() -> HealthResponse:
        return HealthResponse(
            config_loaded=bool(settings.declarant.nif),
            certificate_configured=settings.certificate.is_configured,
            quipu_configured=settings.quipu.is_configured,
            testing_mode=settings.testing,
        )

    # Global exception handler: catch ValueError/InvalidOperation from data model
    # validation (NIF, IBAN, Decimal parsing) and return clean 422 JSON errors.
    @application.exception_handler(ValueError)
    async def value_error_handler(request: Request, exc: ValueError) -> JSONResponse:
        log.warning("validation_error", detail=str(exc), path=request.url.path)
        return JSONResponse(
            status_code=422,
            content={"error": "validation_error", "detail": str(exc)},
        )

    @application.exception_handler(InvalidOperation)
    async def decimal_error_handler(request: Request, exc: InvalidOperation) -> JSONResponse:
        log.warning("decimal_error", detail=str(exc), path=request.url.path)
        return JSONResponse(
            status_code=422,
            content={"error": "validation_error", "detail": f"Invalid decimal value: {exc}"},
        )

    application.include_router(generate_router)
    application.include_router(submit_router)
    application.include_router(quipu_router)
    application.include_router(simulate_router)

    return application


def get_submit_client() -> PresentacionDirectaClient | None:
    """Get the shared submission client (created at startup)."""
    return _submit_client


# Default app instance for `uvicorn aeat_autonomo.api:app`
app = create_app()
