"""Tests for fclone_driver: python3 -m unittest discover -s ci"""

import unittest

import fclone_driver

LITE = "2 bytes:\n  t/one\n  t/a/two\n\n2 bytes:\n  t/four\n  t/a/b/three\n"


class Groups(unittest.TestCase):
    def test_order_within_and_between_groups_is_ignored(self) -> None:
        fast = "2 bytes:\n  t/a/b/three\n  t/four\n\n2 bytes:\n  t/a/two\n  t/one\n"
        self.assertEqual(fclone_driver.groups(fast), fclone_driver.groups(LITE))

    def test_windows_separators_and_line_ends_are_one_spelling(self) -> None:
        lite = LITE.replace("/", "\\").replace("\n", "\r\n")
        self.assertEqual(fclone_driver.groups(lite), fclone_driver.groups(LITE))

    def test_a_missing_or_moved_path_differs(self) -> None:
        moved = LITE.replace("t/four", "t/five")
        self.assertNotEqual(fclone_driver.groups(moved), fclone_driver.groups(LITE))
        dropped = LITE.split("\n\n")[0] + "\n"
        self.assertNotEqual(fclone_driver.groups(dropped), fclone_driver.groups(LITE))

    def test_a_size_is_part_of_the_group(self) -> None:
        other = LITE.replace("2 bytes:", "3 bytes:", 1)
        self.assertNotEqual(fclone_driver.groups(other), fclone_driver.groups(LITE))

    def test_no_output_is_no_groups(self) -> None:
        self.assertEqual(fclone_driver.groups(""), frozenset())


class Compare(unittest.TestCase):
    def test_matching_runs_report_nothing(self) -> None:
        got = fclone_driver.compare(LITE, LITE, expected=2)
        self.assertEqual(got, [])

    def test_a_wrong_group_count_is_reported(self) -> None:
        got = fclone_driver.compare(LITE, LITE, expected=3)
        self.assertEqual(len(got), 1)
        self.assertIn("expected 3 groups", got[0])

    def test_differing_groups_are_reported(self) -> None:
        fast = LITE.replace("t/four", "t/five")
        got = fclone_driver.compare(fast, LITE, expected=2)
        self.assertEqual(len(got), 1)
        self.assertIn("fclone_fast", got[0])


if __name__ == "__main__":
    unittest.main()
