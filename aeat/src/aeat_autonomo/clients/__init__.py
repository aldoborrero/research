"""External service clients (AEAT, Quipu)."""

from .aeat import PresentacionDirectaClient, SubmissionResult, TGVIOnlineClient
from .quipu import QuarterlyTotals, QuipuClient

__all__ = [
    "PresentacionDirectaClient",
    "QuarterlyTotals",
    "QuipuClient",
    "SubmissionResult",
    "TGVIOnlineClient",
]
