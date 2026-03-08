"""FastAPI application for AEAT tax automation.

Usage:
    uvicorn aeat_autonomo.api:app --host 0.0.0.0 --port 8000

Or via CLI:
    aeat serve --host 0.0.0.0 --port 8000
"""

from __future__ import annotations

from fastapi import FastAPI

from .auth import configure_auth
from .config import ApiSettings, load_settings
from .routes_generate import router as generate_router
from .routes_quipu import router as quipu_router
from .routes_simulate import router as simulate_router
from .routes_submit import router as submit_router
from .schemas import HealthResponse

# Module-level settings, populated at startup
_settings: ApiSettings = ApiSettings()


def create_app(settings: ApiSettings | None = None) -> FastAPI:
    """Create and configure the FastAPI application."""
    global _settings  # noqa: PLW0603

    if settings is None:
        settings = load_settings()
    _settings = settings

    if settings.api_key:
        configure_auth(settings.api_key)

    application = FastAPI(
        title="AEAT Autonomo API",
        description=(
            "REST API for Spanish AEAT tax form generation and submission. "
            "Designed for integration with n8n and other automation platforms."
        ),
        version="0.1.0",
    )

    # Health check (no auth required)
    @application.get("/health", response_model=HealthResponse, tags=["health"])
    def health() -> HealthResponse:
        return HealthResponse(
            config_loaded=bool(settings.declarant_nif),
            certificate_configured=bool(settings.cert_path and settings.cert_password),
            quipu_configured=bool(settings.quipu_key and settings.quipu_secret),
            testing_mode=settings.testing,
        )

    application.include_router(generate_router)
    application.include_router(submit_router)
    application.include_router(quipu_router)
    application.include_router(simulate_router)

    return application


# Default app instance for `uvicorn aeat_autonomo.api:app`
app = create_app()
