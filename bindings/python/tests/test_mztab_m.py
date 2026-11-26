"""Tests of the Python package (run with pytest, or `python -m unittest`)."""

import pathlib
import types
import unittest

import mztab_m

ROOT = pathlib.Path(__file__).resolve().parents[3]
BASE = (ROOT / "crates/mztab-m/tests/golden/full.canonical.mztab").read_text(encoding="utf-8")
BASE_JSON = (ROOT / "crates/mztab-m/tests/golden/full.json").read_text(encoding="utf-8")


class Validate(unittest.TestCase):
    def test_the_base_document_is_valid(self):
        self.assertEqual(mztab_m.validate(BASE), [])
        self.assertEqual(mztab_m.validate(BASE.encode()), [], "bytes")
        self.assertEqual(mztab_m.validate(bytearray(BASE.encode())), [], "bytearray")
        self.assertEqual(mztab_m.validate(BASE_JSON), [], "JSON")

    def test_findings_are_dicts(self):
        bad = BASE.replace("\t2.1.0-M\n", "\t2.1\n", 1)
        (finding,) = mztab_m.validate(bad)
        self.assertEqual(finding["code"], "MZT-S006")
        self.assertEqual(finding["severity"], "error")
        self.assertEqual(finding["layer"], "syntax")
        self.assertEqual((finding["line"], finding["column"], finding["cell"]), (1, 22, 3))
        self.assertTrue(finding["spec"].startswith("https://github.com/HUPO-PSI/mzTab-M/blob/"))
        self.assertIsNotNone(finding["detail"])

    def test_options(self):
        missing = "MTD\tmzTab-version\t2.1.0-M\n"
        structure = mztab_m.validate(missing, layer="structure")
        self.assertTrue(any(f["code"] == "MZT-T009" for f in structure))
        self.assertEqual(mztab_m.validate(missing, layer="syntax"), [])
        quiet = mztab_m.validate(missing, severities={"MZT-T009": "off", "MZT-T011": "off"})
        self.assertFalse(any(f["code"] == "MZT-T009" for f in quiet))
        proxy_quiet = mztab_m.validate(
            missing,
            severities=types.MappingProxyType({"MZT-T009": "off", "MZT-T011": "off"}),
        )
        self.assertFalse(any(f["code"] == "MZT-T009" for f in proxy_quiet))
        obo = "[Term]\nid: MS:1\nname: x\n"
        terms = mztab_m.validate(BASE, vocabularies=[obo])
        self.assertTrue(any(f["code"] == "MZT-X004" for f in terms))
        for bad in [
            dict(layer="all"),
            dict(severities={"MZT-T009": "loud"}),
            dict(severities={"MZT-T009=off": "off"}),
            dict(format="xml"),
        ]:
            with self.assertRaises(ValueError):
                mztab_m.validate(BASE, **bad)
        with self.assertRaises(TypeError):
            mztab_m.validate(BASE, vocabularies=obo)
        with self.assertRaises(TypeError):
            mztab_m.validate(BASE, vocabularies=b"[Term]\n")
        with self.assertRaises(TypeError):
            mztab_m.validate([77, 84, 68])
        with self.assertRaises(UnicodeEncodeError):
            mztab_m.validate("\ud800")
        with self.assertRaises(TypeError):
            mztab_m.validate(3)


class Convert(unittest.TestCase):
    def test_both_ways(self):
        self.assertEqual(mztab_m.convert(BASE, "json"), BASE_JSON.rstrip("\n"))
        self.assertEqual(mztab_m.convert(BASE_JSON, "tsv"), BASE)
        self.assertEqual(mztab_m.canonical(BASE), BASE)
        self.assertEqual(mztab_m.canonical(BASE_JSON, format="json"), BASE_JSON.rstrip("\n"))

    def test_lossy(self):
        unknown = BASE.replace("MTD\ttitle\t", "MTD\tnot-a-key\tx\nMTD\ttitle\t", 1)
        with self.assertRaises(ValueError):
            mztab_m.convert(unknown, "json")
        self.assertEqual(mztab_m.convert(unknown, "json", lossy=True), BASE_JSON.rstrip("\n"))


class Inspect(unittest.TestCase):
    def test_facts(self):
        facts = mztab_m.inspect(BASE)
        self.assertEqual(facts["version"], "2.1.0-M")
        self.assertEqual(facts["sml_rows"], 2)
        self.assertIsNone(mztab_m.inspect("MTD\ttitle\tx\n")["sml_rows"])

    def test_codes_and_constants(self):
        codes = mztab_m.codes()
        self.assertTrue(any(c["code"] == "MZT-T009" and c["requirement"] == "MUST" for c in codes))
        self.assertEqual(len(mztab_m.SPEC_COMMIT), 40)
        self.assertEqual(mztab_m.SPEC_VERSION, "2.1.0-M")
        self.assertEqual(mztab_m.vocabulary_size("[Term]\nid: MS:1\n"), 1)


if __name__ == "__main__":
    unittest.main()
