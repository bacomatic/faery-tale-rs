"""Tests for tools/extract_text.py (T1.4 narrative text from src/narr.asm)."""
import sys
from pathlib import Path

import pytest

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

import extract_text as et  # noqa: E402

SRC = TOOLS.parent / "src"


@pytest.fixture(scope="module")
def ex():
    return et.extract(SRC)


def test_counts(ex):
    assert {k: len(v["strings"]) for k, v in ex.strings.items()} == {
        "event_msg": 39, "place_msg": 27, "inside_msg": 23, "speeches": 61}
    assert len(ex.question["rows"]) == 8
    assert len(ex.placard_text["entries"]) == 20
    assert len(ex.tables["place_tbl"]["rows"]) == 29
    assert len(ex.tables["inside_tbl"]["rows"]) == 37


def test_empty_slots_keep_alignment(ex):
    # narr.asm:165-166, 200-201 (dc.b 0 placeholders), 502 (speech 52)
    assert ex.strings["place_msg"]["strings"][:3] == ["", "", "% returned to the village of Tambry."]
    assert ex.strings["inside_msg"]["strings"][:3] == ["", "", "% came to a small chamber."]
    assert ex.strings["speeches"]["strings"][52] == ""
    assert ex.strings["speeches"]["strings"][53].startswith('"The dragon\'s cave is east')


def test_commented_out_entry_absent(ex):
    # narr.asm:197 is fully commented out
    assert "He entered the garden area." not in ex.strings["place_msg"]["strings"]
    assert ex.strings["place_msg"]["strings"][-1] == "% found a cave in the hillside."


def test_multiline_and_embedded_quotes(ex):
    s = ex.strings["speeches"]["strings"]
    assert s[9] == '"Nice weather we\'re having, isn\'t it?" queried the ranger.'  # narr.asm:369-371
    assert ex.strings["event_msg"]["strings"][34] == '"They\'re all dead!" he cried.'  # narr.asm:52
    assert s[15] == ('"State your business!" said the guard.\r'
                     '"My business is with the king." stated %, respectfully.')  # narr.asm:385-386


def test_comment_split_respects_quotes():
    assert et.split_comment("dc.b 'a;b',0 ; note") == ("dc.b 'a;b',0 ", "note")
    assert et.split_comment(";\t\tdc.b \"x\",0") == ("", 'dc.b "x",0')


def test_questions_in_qq_order(ex):
    rows = ex.question["rows"]
    assert [r["label"] for r in rows] == [f"q{i}" for i in range(1, 9)]
    assert rows[0]["question"] == "To Quest for the...?"  # narr.asm:74
    assert rows[7]["question"] == "In black darker than...?"  # narr.asm:81
    # fmain2.c:1307, indexed by the same j as question(j) (fmain2.c:1316-1317)
    assert [r["answer"] for r in rows] == [
        "LIGHT", "HEED", "DEED", "SIGHT", "FLIGHT", "CREED", "BLIGHT", "NIGHT"]


def test_placard_decoding(ex):
    e = ex.placard_text["entries"]
    assert [x["label"] for x in e[:8]] == ["msg1", "msg2", "msg3", "msg4", "msg5", "msg6", "msg7", "msg7a"]
    first = e[0]["segments"][0]  # narr.asm:251: XY,20/2,28,'   "Rescue the Talisman!"'
    assert first == {"x_half": 10, "x": 20, "y": 28, "text": '   "Rescue the Talisman!"'}
    assert e[8]["segments"] == [{"x_half": 10, "x": 20, "y": 26, "text": ""}]  # msg8: XY,21/2,26,ETX
    assert e[9]["segments"][0] == {"x_half": None, "x": None, "y": None, "text": " had rescued Katra,"}
    assert e[19]["segments"][0]["x"] == 128  # msg12: XY,128/2,19


def test_decode_ssp_rejects_missing_etx():
    with pytest.raises(ValueError):
        et.decode_ssp(b"abc")


def test_tables(ex):
    pt = ex.tables["place_tbl"]["rows"]
    assert pt[0] == {"lo": 51, "hi": 51, "msg_index": 19, "comment": "small keep", "line": 87}
    assert pt[-1] == {"lo": 0, "hi": 255, "msg_index": 0, "comment": "nil", "line": 115}
    it = ex.tables["inside_tbl"]["rows"]
    assert it[5] == {"lo": 30, "hi": 30, "msg_index": 7, "comment": "keep interior - ITEMIZE??", "line": 123}
    # every referenced index resolves to an entry in the matching message list
    assert all(r["msg_index"] < 27 for r in pt) and all(r["msg_index"] < 23 for r in it)


def test_no_dollar_placeholder(ex):
    # tools/review/PLAN.md: `$` is documented in narr.asm:4 but extract() never substitutes it
    assert not any("$" in s for v in ex.strings.values() for s in v["strings"])
