"""API key authentication dependency."""

from __future__ import annotations

from fastapi import HTTPException, Security
from fastapi.security import APIKeyHeader

_api_key_header = APIKeyHeader(name="X-API-Key", auto_error=False)

# Set at startup by the app factory
_expected_key: str = ""


def configure_auth(api_key: str) -> None:
    """Set the expected API key. Called once at startup."""
    global _expected_key  # noqa: PLW0603
    _expected_key = api_key


async def require_api_key(
    key: str | None = Security(_api_key_header),
) -> str:
    """FastAPI dependency that validates the X-API-Key header."""
    if not _expected_key:
        # No key configured — auth disabled (localhost/dev mode)
        return ""
    if not key or key != _expected_key:
        raise HTTPException(status_code=401, detail="Invalid or missing API key")
    return key
