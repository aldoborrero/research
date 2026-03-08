"""BOE (Boletín Oficial del Estado) fixed-width file generator.

AEAT uses fixed-width ASCII files encoded in ISO-8859-1 for most tax models.
Each record has a fixed length with fields at specific positions. This module
provides a base class for generating these files.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class BOEField:
    """A single field in a BOE record."""

    name: str
    position: int  # 1-based start position
    length: int
    field_type: str  # "AN" (alphanumeric), "N" (numeric), "NUM" (numeric with sign)

    @property
    def end_position(self) -> int:
        return self.position + self.length - 1


@dataclass
class BOERecord:
    """A single record (line) in a BOE file."""

    fields: list[BOEField] = field(default_factory=list)
    _values: dict[str, str] = field(default_factory=dict)

    def set(self, name: str, value: str | int | float) -> None:
        """Set a field value."""
        self._values[name] = str(value)

    def render(self) -> str:
        """Render the record as a fixed-width string."""
        # Find total record length
        if not self.fields:
            return ""
        total_length = max(f.end_position for f in self.fields)
        buf = [" "] * total_length

        for fld in self.fields:
            raw = self._values.get(fld.name, "")
            if fld.field_type == "N" or fld.field_type == "NUM":
                # Right-justified, zero-padded
                formatted = raw.replace(".", "").replace("-", "").rjust(fld.length, "0")
            else:
                # Left-justified, space-padded
                formatted = raw.ljust(fld.length)

            # Truncate to field length
            formatted = formatted[: fld.length]

            start = fld.position - 1
            for i, ch in enumerate(formatted):
                buf[start + i] = ch

        return "".join(buf)


def encode_boe(content: str) -> bytes:
    """Encode BOE content to ISO-8859-1 as required by AEAT."""
    return content.encode("iso-8859-1")


def format_amount(amount: int, length: int = 17) -> str:
    """Format an amount in cents for BOE fields.

    AEAT expects amounts as integers (cents), right-justified, zero-padded.
    Negative amounts use 'N' prefix instead of '-'.

    Args:
        amount: Amount in cents (e.g., 12345 for €123.45).
        length: Total field length including sign character.
    """
    if amount < 0:
        return "N" + str(abs(amount)).rjust(length - 1, "0")
    return str(amount).rjust(length, "0")


def format_nif(nif: str, length: int = 9) -> str:
    """Format a NIF/CIF for BOE fields. Right-justified, zero-padded on the left."""
    return nif.strip().upper().rjust(length, "0")
