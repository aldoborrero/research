"""BOE (Boletín Oficial del Estado) fixed-width file generator.

AEAT uses fixed-width ASCII files encoded in ISO-8859-1 for most tax models.
Each record has a fixed length with fields at specific positions. This module
provides shared formatting helpers and a generic record class.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from decimal import Decimal, ROUND_HALF_UP


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


def cents(amount: Decimal) -> int:
    """Convert a Decimal euro amount to integer cents."""
    return int((amount * 100).to_integral_value(rounding=ROUND_HALF_UP))


def num(amount: int, length: int = 17) -> str:
    """Format unsigned numeric field: right-justified, zero-padded, truncated."""
    return str(max(0, amount)).rjust(length, "0")[:length]


def signed(amount: int, length: int = 17) -> str:
    """Format signed numeric field.

    First position: blank (positive/zero) or 'N' (negative).
    Remaining: absolute value, right-justified, zero-padded.
    """
    if amount < 0:
        return "N" + str(abs(amount)).rjust(length - 1, "0")[:length - 1]
    return " " + str(amount).rjust(length - 1, "0")[:length - 1]


def pct(rate: Decimal) -> str:
    """Format percentage as 5-char field (3 integer + 2 decimal, no separator)."""
    c = int((rate * 100).to_integral_value(rounding=ROUND_HALF_UP))
    return str(c).rjust(5, "0")[:5]


def an(value: str, length: int) -> str:
    """Format alphanumeric field: left-justified, space-padded, uppercase."""
    return value.upper().ljust(length)[:length]


def bool_yn(value: bool) -> str:
    """Format boolean as '1' (yes) or '2' (no)."""
    return "1" if value else "2"


def encode_boe(content: str) -> bytes:
    """Encode BOE content to ISO-8859-1 as required by AEAT."""
    return content.encode("iso-8859-1")


def format_amount(amount: int, length: int = 17) -> str:
    """Format an amount in cents for BOE fields.

    AEAT expects amounts as integers (cents), right-justified, zero-padded.
    Negative amounts use 'N' prefix instead of '-'.

    Unlike ``signed()``, positive values are fully zero-padded (no space prefix).
    This matches the format used in unsigned-but-signable fields.
    """
    if amount < 0:
        return "N" + str(abs(amount)).rjust(length - 1, "0")
    return str(amount).rjust(length, "0")


def format_nif(nif: str, length: int = 9) -> str:
    """Format a NIF/CIF for BOE fields. Right-justified, zero-padded on the left."""
    return nif.strip().upper().rjust(length, "0")


# --- Validation helpers ---

_NIF_RE = re.compile(r"^[0-9]{8}[A-Z]$")
_NIE_RE = re.compile(r"^[XYZ][0-9]{7}[A-Z]$")
_CIF_RE = re.compile(r"^[A-H][0-9]{7}[0-9A-J]$")
_NIF_LETTERS = "TRWAGMYFPDXBNJZSQVHLCKE"


def validate_nif(nif: str) -> str:
    """Validate and normalize a Spanish NIF/NIE/CIF.

    Returns the normalized (uppercase, stripped) NIF.
    Raises ValueError if the format is invalid.
    """
    nif = nif.strip().upper()
    if _NIF_RE.match(nif):
        # Standard NIF: check letter
        expected = _NIF_LETTERS[int(nif[:8]) % 23]
        if nif[8] != expected:
            raise ValueError(f"Invalid NIF check letter: expected {expected}, got {nif[8]}")
        return nif
    if _NIE_RE.match(nif):
        # NIE: replace leading letter with digit, then check like NIF
        prefix_map = {"X": "0", "Y": "1", "Z": "2"}
        num_str = prefix_map[nif[0]] + nif[1:8]
        expected = _NIF_LETTERS[int(num_str) % 23]
        if nif[8] != expected:
            raise ValueError(f"Invalid NIE check letter: expected {expected}, got {nif[8]}")
        return nif
    if _CIF_RE.match(nif):
        return nif
    raise ValueError(
        f"Invalid NIF/NIE/CIF format: {nif!r}. "
        "Expected 8 digits + letter (NIF), X/Y/Z + 7 digits + letter (NIE), "
        "or letter + 7 digits + control (CIF)."
    )


_IBAN_ES_RE = re.compile(r"^ES[0-9]{22}$")


def validate_iban(iban: str) -> str:
    """Validate a Spanish IBAN (ES + 22 digits).

    Returns the normalized IBAN (uppercase, no spaces).
    Raises ValueError if invalid. Empty string is allowed (optional field).
    """
    normalized = iban.replace(" ", "").strip().upper()
    if not normalized:
        return ""
    if not _IBAN_ES_RE.match(normalized):
        raise ValueError(
            f"Invalid Spanish IBAN: {iban!r}. Expected format: ES + 22 digits."
        )
    # IBAN mod-97 check
    rearranged = normalized[4:] + normalized[:4]
    numeric = "".join(str(int(c, 36)) for c in rearranged)
    if int(numeric) % 97 != 1:
        raise ValueError(f"Invalid IBAN check digits: {iban!r}")
    return normalized
