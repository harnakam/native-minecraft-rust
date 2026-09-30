import tempfile
import unittest
from pathlib import Path
from compare_container_transfer import read


class ComparisonIntegrity(unittest.TestCase):
    def test_rejects_empty_partial_and_duplicate_observations(self):
        for data in ('', 'transfer chest 0 1 1 0 -\n',
                     'transfer chest 0 1 1 0 -\n' * 2):
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / 'observations.txt'
                path.write_text(data, encoding='utf-8')
                with self.assertRaises(ValueError):
                    read(path)


if __name__ == '__main__':
    unittest.main()
