"""AEAT tax model generators (BOE fixed-width files)."""

from .modelo130 import Modelo130Data, generate_130_boe
from .modelo303 import Modelo303Data, generate_303_boe

__all__ = [
    "Modelo130Data",
    "Modelo303Data",
    "generate_130_boe",
    "generate_303_boe",
]
