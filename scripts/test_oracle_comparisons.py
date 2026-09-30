"""Protect behavioral comparisons against truncated, duplicate and non-finite evidence."""
import tempfile
from pathlib import Path
import unittest

import compare_collision
import compare_travel


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

    def test_union_comparison_preserves_volume(self):
        cube = [(0, 0, 0, 1, 1, 1)]
        halves = [(0, 0, 0, 0.5, 1, 1), (0.5, 0, 0, 1, 1, 1)]
        self.assertTrue(compare_collision.same(cube, halves))
        self.assertFalse(compare_collision.same(cube, halves[:1]))
        self.assertFalse(compare_collision.same(cube, []))
        self.assertTrue(compare_collision.same([], [(0, 0, 0, 1, 0, 1)]))


if __name__ == '__main__':
    unittest.main()
