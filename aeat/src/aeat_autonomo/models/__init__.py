"""AEAT tax model generators (BOE fixed-width files)."""

from .boe import encode_boe
from .modelo_130 import Modelo130Data, generate_130_boe
from .modelo_303 import Modelo303Data, generate_303_boe

__all__ = [
    "Modelo130Data",
    "Modelo303Data",
    "encode_boe",
    "generate_130_boe",
    "generate_303_boe",
]
