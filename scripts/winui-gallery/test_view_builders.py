"""Protect the existing declarative API when opt-in types share a class name."""

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "generate_view_builders.py"
SPEC = importlib.util.spec_from_file_location("gallery_view_builders", SCRIPT)
BUILDERS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILDERS)


class NamespaceIsolationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.model = BUILDERS.Model()
        for directory, crate in BUILDERS.CRATES:
            BUILDERS.read_paths(cls.model, directory, crate)
        for directory, crate in BUILDERS.CRATES:
            BUILDERS.read_classes(cls.model, directory, crate)
        cls.output = BUILDERS.generate(cls.model)

    def assert_output_contains(self, declaration):
        self.assertTrue(declaration in self.output, f"Missing generated declaration: {declaration}")

    def test_existing_and_projected_pages_keep_distinct_type_paths(self):
        self.assertEqual(BUILDERS.class_path(self.model, "Page"), "ferroui_controls::Page")
        self.assertEqual(BUILDERS.class_path(self.model, "WinUIPage"), "ferroui_controls::winui::Page")
        self.assertNotIn("Page", self.model.ambiguous)
        # The existing Page is abstract; ContentPage keeps that original base.
        self.assertEqual(self.model.bases["ContentPage"], "Page")
        self.assert_output_contains("pub fn content_page() -> El<ferroui_controls::ContentPage>")
        self.assert_output_contains("pub fn win_ui_page() -> El<ferroui_controls::winui::Page>")
        self.assert_output_contains("pub use ferroui_controls::winui::Page as WinUIPage;")

    def test_navigation_cache_property_belongs_only_to_projected_page(self):
        native = {entry[1] for entry in self.model.properties.get("Page", [])}
        winui = {entry[1] for entry in self.model.properties.get("WinUIPage", [])}
        self.assertNotIn("navigation_cache_mode", native)
        self.assertIn("navigation_cache_mode", winui)
        self.assert_output_contains("pub trait PageProps")
        self.assert_output_contains("pub trait WinUIPageProps")

    def test_projected_property_enum_resolves_to_opt_in_namespace(self):
        self.assertEqual(
            BUILDERS.resolve(self.model, "WinUINumberBoxSpinButtonPlacementMode"),
            "ferroui_controls::winui::NumberBoxSpinButtonPlacementMode",
        )
        self.assert_output_contains("pub fn numeric_up_down() -> El<ferroui_controls::NumericUpDown>")
        self.assert_output_contains("pub fn win_ui_number_box() -> El<ferroui_controls::winui::NumberBox>")


if __name__ == "__main__":
    unittest.main()
