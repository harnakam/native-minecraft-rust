import tempfile
import unittest
from pathlib import Path
from compare_item_properties import read


class ComparisonIntegrity(unittest.TestCase):
    def parse(self, value):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'observations.txt'
            path.write_text(value, encoding='utf-8')
            return read(path)

    def test_rejects_empty_and_partial_registry(self):
        for value in ('', 'item_property 1 64 false 0\n'):
            with self.assertRaises(ValueError):
                self.parse(value)

    def test_rejects_duplicates_and_invalid_properties(self):
        for value in ('item_property 1 64 false 0\n' * 2,
                      'item_property 1 65 false 0\n',
                      'item_property 1 64 unknown 0\n'):
            with self.assertRaises(ValueError):
                self.parse(value)


if __name__ == '__main__':
    unittest.main()
