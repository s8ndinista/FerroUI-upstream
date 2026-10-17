//! The pages of the gallery (namespace `WinUIThemeGallery.Pages`): one page per family of the
//! controls the theme styles, each a document (`<Page>.xaml`) with its class (`<page>.rs`), and
//! beside it the native WinUI 3 page with the same controls in the same layout
//! (`<Page>.winui.xml`), which the comparison host of Windows loads with `XamlReader.Load`.
//!
//! [`PAGES`] is the table of the pages, in the order of the list of the window: what the window,
//! the capture mode and the comparison host show.

use crate::markup::XamlClass;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{Ref, TypeInfo};
use ferroui_controls::Control;

mod buttons_page;
mod toggles_page;
mod text_input_page;
mod number_input_page;
mod combo_box_page;
mod lists_page;
mod range_page;
mod date_time_page;
mod menus_page;
mod flyouts_page;
mod tabs_page;
mod containers_page;
mod navigation_samples_page;
mod table_page;
mod dialogs_page;
mod text_calibration_page;

pub use buttons_page::ButtonsPage;
pub use toggles_page::TogglesPage;
pub use text_input_page::TextInputPage;
pub use number_input_page::NumberInputPage;
pub use combo_box_page::ComboBoxPage;
pub use lists_page::ListsPage;
pub use range_page::RangePage;
pub use date_time_page::DateTimePage;
pub use menus_page::MenusPage;
pub use flyouts_page::FlyoutsPage;
pub use tabs_page::TabsPage;
pub use containers_page::ContainersPage;
pub use navigation_samples_page::NavigationSamplesPage;
pub use table_page::TablePage;
pub use dialogs_page::DialogsPage;
pub use text_calibration_page::TextCalibrationPage;
pub use table_page::TableRow;

/// A page of the gallery.
#[derive(Debug)]
pub struct PageInfo {
    /// The name of the page in the names of the pictures and in the options
    /// (`FERROUI_THEME_GALLERY_PAGES`).
    pub id: &'static str,
    /// The title of the page in the list of the window.
    pub title: &'static str,
    /// The rooted asset path of the document of the page.
    pub document: &'static str,
    /// The path of the native page below the directory of the crate.
    pub native_file: &'static str,
    /// The native page: WinUI XAML with the same controls in the same layout.
    pub native: &'static str,
    /// Creates the page (its constructor loads its document, and panics if it does not load).
    pub create: fn() -> Ref<Control>,
}

impl PageInfo {
    /// The page with the id `id` (without regard to case).
    pub fn find(id: &str) -> Option<&'static PageInfo> {
        PAGES.iter().find(|page| page.id.eq_ignore_ascii_case(id.trim()))
    }
}

/// The pages, in the order of the list of the window.
pub static PAGES: &[PageInfo] = &[
    PageInfo {
        id: "buttons",
        title: "Buttons",
        document: ButtonsPage::DOCUMENT_PATH,
        native_file: "Pages/ButtonsPage.winui.xml",
        native: include_str!("ButtonsPage.winui.xml"),
        create: buttons_page::create,
    },
    PageInfo {
        id: "toggles",
        title: "Check boxes and toggles",
        document: TogglesPage::DOCUMENT_PATH,
        native_file: "Pages/TogglesPage.winui.xml",
        native: include_str!("TogglesPage.winui.xml"),
        create: toggles_page::create,
    },
    PageInfo {
        id: "text",
        title: "Text input",
        document: TextInputPage::DOCUMENT_PATH,
        native_file: "Pages/TextInputPage.winui.xml",
        native: include_str!("TextInputPage.winui.xml"),
        create: text_input_page::create,
    },
    PageInfo {
        id: "numbers",
        title: "Numbers and suggestions",
        document: NumberInputPage::DOCUMENT_PATH,
        native_file: "Pages/NumberInputPage.winui.xml",
        native: include_str!("NumberInputPage.winui.xml"),
        create: number_input_page::create,
    },
    PageInfo {
        id: "combobox",
        title: "Combo box",
        document: ComboBoxPage::DOCUMENT_PATH,
        native_file: "Pages/ComboBoxPage.winui.xml",
        native: include_str!("ComboBoxPage.winui.xml"),
        create: combo_box_page::create,
    },
    PageInfo {
        id: "lists",
        title: "Lists and trees",
        document: ListsPage::DOCUMENT_PATH,
        native_file: "Pages/ListsPage.winui.xml",
        native: include_str!("ListsPage.winui.xml"),
        create: lists_page::create,
    },
    PageInfo {
        id: "range",
        title: "Sliders, progress and scrolling",
        document: RangePage::DOCUMENT_PATH,
        native_file: "Pages/RangePage.winui.xml",
        native: include_str!("RangePage.winui.xml"),
        create: range_page::create,
    },
    PageInfo {
        id: "datetime",
        title: "Date and time",
        document: DateTimePage::DOCUMENT_PATH,
        native_file: "Pages/DateTimePage.winui.xml",
        native: include_str!("DateTimePage.winui.xml"),
        create: date_time_page::create,
    },
    PageInfo {
        id: "menus",
        title: "Menus and command bars",
        document: MenusPage::DOCUMENT_PATH,
        native_file: "Pages/MenusPage.winui.xml",
        native: include_str!("MenusPage.winui.xml"),
        create: menus_page::create,
    },
    PageInfo {
        id: "flyouts",
        title: "Flyouts, tool tips and notifications",
        document: FlyoutsPage::DOCUMENT_PATH,
        native_file: "Pages/FlyoutsPage.winui.xml",
        native: include_str!("FlyoutsPage.winui.xml"),
        create: flyouts_page::create,
    },
    PageInfo {
        id: "tabs",
        title: "Tabs",
        document: TabsPage::DOCUMENT_PATH,
        native_file: "Pages/TabsPage.winui.xml",
        native: include_str!("TabsPage.winui.xml"),
        create: tabs_page::create,
    },
    PageInfo {
        id: "containers",
        title: "Containers",
        document: ContainersPage::DOCUMENT_PATH,
        native_file: "Pages/ContainersPage.winui.xml",
        native: include_str!("ContainersPage.winui.xml"),
        create: containers_page::create,
    },
    PageInfo {
        id: "navigation",
        title: "Pages and navigation",
        document: NavigationSamplesPage::DOCUMENT_PATH,
        native_file: "Pages/NavigationSamplesPage.winui.xml",
        native: include_str!("NavigationSamplesPage.winui.xml"),
        create: navigation_samples_page::create,
    },
    PageInfo {
        id: "table",
        title: "Table view",
        document: TablePage::DOCUMENT_PATH,
        native_file: "Pages/TablePage.winui.xml",
        native: include_str!("TablePage.winui.xml"),
        create: table_page::create,
    },
    PageInfo {
        id: "dialogs",
        title: "Dialogs",
        document: DialogsPage::DOCUMENT_PATH,
        native_file: "Pages/DialogsPage.winui.xml",
        native: include_str!("DialogsPage.winui.xml"),
        create: dialogs_page::create,
    },
    PageInfo {
        id: "textcal",
        title: "Text calibration",
        document: TextCalibrationPage::DOCUMENT_PATH,
        native_file: "Pages/TextCalibrationPage.winui.xml",
        native: include_str!("TextCalibrationPage.winui.xml"),
        create: text_calibration_page::create,
    },
];

/// The classes of the namespace `WinUIThemeGallery.Pages` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[
    ButtonsPage::TYPE,
    TogglesPage::TYPE,
    TextInputPage::TYPE,
    NumberInputPage::TYPE,
    ComboBoxPage::TYPE,
    ListsPage::TYPE,
    RangePage::TYPE,
    DateTimePage::TYPE,
    MenusPage::TYPE,
    FlyoutsPage::TYPE,
    TabsPage::TYPE,
    ContainersPage::TYPE,
    NavigationSamplesPage::TYPE,
    TablePage::TYPE,
    DialogsPage::TYPE,
    TextCalibrationPage::TYPE,
];

/// The classes of the namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[
    &ButtonsPage::XAML_CLASS,
    &TogglesPage::XAML_CLASS,
    &TextInputPage::XAML_CLASS,
    &NumberInputPage::XAML_CLASS,
    &ComboBoxPage::XAML_CLASS,
    &ListsPage::XAML_CLASS,
    &RangePage::XAML_CLASS,
    &DateTimePage::XAML_CLASS,
    &MenusPage::XAML_CLASS,
    &FlyoutsPage::XAML_CLASS,
    &TabsPage::XAML_CLASS,
    &ContainersPage::XAML_CLASS,
    &NavigationSamplesPage::XAML_CLASS,
    &TablePage::XAML_CLASS,
    &DialogsPage::XAML_CLASS,
    &TextCalibrationPage::XAML_CLASS,
];

/// The types of the namespace declared with `ferro_markup_type!` (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[<TableRow as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ferroui_base::data::core::ValueTypes::register_reference::<TableRow>();
}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`): none.
pub(crate) fn register_lists() {}
