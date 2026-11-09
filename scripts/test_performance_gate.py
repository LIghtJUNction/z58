import unittest

from performance_gate import evaluate


def fixture():
    return [
        {"algorithm": algorithm, "bytes": size, "operation": operation,
         "pattern": "random", "sample": sample, "iterations": 1000,
         "ns_per_op": duration}
        for size in (32, 64)
        for operation in ("encode", "decode")
        for sample in range(9)
        for algorithm, duration in (("z58-fixed", 10), ("five8", 30), ("braid58", 20))
    ]


class PerformanceGateTests(unittest.TestCase):
    def test_strongest_baseline_decides_every_case(self):
        result = evaluate(fixture())
        self.assertEqual(len(result), 4)
        self.assertTrue(all(row["passed"] for row in result))
        self.assertTrue(all(row["strongest_baseline"] == "braid58" for row in result))
        self.assertTrue(all(row["speedup"] == 2 for row in result))

    def test_one_failed_case_is_preserved(self):
        rows = fixture()
        for row in rows:
            if row["bytes"] == 64 and row["operation"] == "decode" and row["algorithm"] == "braid58":
                row["ns_per_op"] = 19
        self.assertEqual(sum(row["passed"] for row in evaluate(rows)), 3)

    def test_missing_baseline_is_rejected(self):
        with self.assertRaises(ValueError):
            evaluate(row for row in fixture() if row["algorithm"] != "braid58")

    def test_smoke_measurements_cannot_be_strict_evidence(self):
        with self.assertRaises(ValueError):
            evaluate(row for row in fixture() if row["sample"] < 2)

    def test_nonfinite_measurements_are_rejected(self):
        rows = fixture()
        rows[0]["ns_per_op"] = float("inf")
        with self.assertRaises(ValueError):
            evaluate(rows)


if __name__ == "__main__":
    unittest.main()
