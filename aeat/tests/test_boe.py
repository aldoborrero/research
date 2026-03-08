"""Tests for boe.py — BOE fixed-width file helpers."""

from aeat_autonomo.models.boe import BOEField, BOERecord, encode_boe, format_amount, format_nif


class TestFormatAmount:
    def test_positive_amount(self):
        assert format_amount(123456) == "00000000000123456"

    def test_zero(self):
        assert format_amount(0) == "00000000000000000"

    def test_negative_amount(self):
        assert format_amount(-123456) == "N0000000000123456"

    def test_small_amount(self):
        assert format_amount(1) == "00000000000000001"

    def test_large_amount(self):
        assert format_amount(999999999999999) == "00999999999999999"

    def test_custom_length(self):
        assert format_amount(42, length=5) == "00042"

    def test_negative_custom_length(self):
        assert format_amount(-42, length=5) == "N0042"

    def test_field_length(self):
        assert len(format_amount(0)) == 17
        assert len(format_amount(-999)) == 17


class TestFormatNif:
    def test_standard_nif(self):
        assert format_nif("12345678Z") == "12345678Z"

    def test_short_nif_padded(self):
        assert format_nif("B1234") == "0000B1234"

    def test_lowercase_uppercased(self):
        assert format_nif("12345678z") == "12345678Z"

    def test_whitespace_stripped(self):
        assert format_nif(" 12345678Z ") == "12345678Z"

    def test_custom_length(self):
        assert format_nif("12345678Z", length=12) == "00012345678Z"


class TestEncodeBoe:
    def test_ascii_content(self):
        result = encode_boe("HELLO")
        assert result == b"HELLO"
        assert isinstance(result, bytes)

    def test_spanish_chars(self):
        result = encode_boe("GARCÍA LÓPEZ")
        assert isinstance(result, bytes)
        # ISO-8859-1 can encode these characters
        assert result.decode("iso-8859-1") == "GARCÍA LÓPEZ"


class TestBOEField:
    def test_end_position(self):
        f = BOEField(name="test", position=10, length=5, field_type="AN")
        assert f.end_position == 14

    def test_end_position_single_char(self):
        f = BOEField(name="x", position=1, length=1, field_type="N")
        assert f.end_position == 1


class TestBOERecord:
    def test_empty_record(self):
        r = BOERecord()
        assert r.render() == ""

    def test_alphanumeric_field_left_justified(self):
        r = BOERecord(fields=[BOEField("name", 1, 10, "AN")])
        r.set("name", "ABC")
        result = r.render()
        assert result == "ABC       "
        assert len(result) == 10

    def test_numeric_field_right_justified(self):
        r = BOERecord(fields=[BOEField("amount", 1, 8, "N")])
        r.set("amount", "123")
        result = r.render()
        assert result == "00000123"

    def test_multiple_fields(self):
        r = BOERecord(fields=[
            BOEField("tag", 1, 3, "AN"),
            BOEField("num", 4, 5, "N"),
        ])
        r.set("tag", "AB")
        r.set("num", "42")
        result = r.render()
        assert result == "AB 00042"
        assert len(result) == 8

    def test_unset_field_defaults(self):
        r = BOERecord(fields=[
            BOEField("alpha", 1, 5, "AN"),
            BOEField("num", 6, 5, "N"),
        ])
        result = r.render()
        assert result == "     00000"

    def test_truncation(self):
        r = BOERecord(fields=[BOEField("short", 1, 3, "AN")])
        r.set("short", "ABCDEF")
        result = r.render()
        assert result == "ABC"
