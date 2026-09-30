"""Protect behavioral comparisons against truncated, duplicate and non-finite evidence."""
import tempfile
from pathlib import Path
import unittest

import compare_collision
import compare_travel
import compare_mining
import compare_minestate
import compare_explosion


class OracleComparisonTests(unittest.TestCase):
    def read(self, reader, text):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'capture.txt'
            path.write_text(text, encoding='utf-8')
            return reader(path)

    def test_travel_rejects_nonfinite_duplicate_and_truncated_records(self):
        valid = 'travel walk 1 0 64 0 0 0 0 true\n'
        for text in (valid + valid, valid.replace('64', 'nan'), valid.replace('64', 'inf'),
                     'travel walk 1 0 64 0 true\n'):
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.read(compare_travel.read, text)

    def test_collision_rejects_duplicate_nonfinite_and_inverted_boxes(self):
        valid = 'shape 1 0 0,0,0,1,1,1\n'
        for text in (valid + valid, 'shape 1 0 0,nan,0,1,1,1\n',
                     'shape 1 0 1,0,0,0,1,1\n', 'shape 1 0 0,0,1\n'):
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.read(compare_collision.read, text)

    def test_explosion_rejects_empty_truncated_and_duplicate_evidence(self):
        for text in ('', 'explosion 0 wire 27\n', 'explosion 0 pos0 0,64,0\n' * 2):
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.read(compare_explosion.read, text)

    def test_union_comparison_preserves_volume(self):
        cube = [(0, 0, 0, 1, 1, 1)]
        halves = [(0, 0, 0, 0.5, 1, 1), (0.5, 0, 0, 1, 1, 1)]
        self.assertTrue(compare_collision.same(cube, halves))
        self.assertFalse(compare_collision.same(cube, halves[:1]))
        self.assertFalse(compare_collision.same(cube, []))
        self.assertTrue(compare_collision.same([], [(0, 0, 0, 1, 0, 1)]))

    def test_mining_allows_actual_zero_hardness_infinity_but_rejects_nan(self):
        self.assertEqual(len(self.read(compare_mining.read, 'hardness normal 165 -1 Infinity\n')),1)
        with self.assertRaises(ValueError):
            self.read(compare_mining.read, 'hardness normal 1 -1 NaN\n')
        valid='minestate stone 1 0.2 true false 0,0,64,0,1\n'
        self.assertEqual(len(self.read(compare_minestate.read, valid)),1)
        with self.assertRaises(ValueError):self.read(compare_minestate.read,valid+valid)


if __name__ == '__main__':
    unittest.main()
