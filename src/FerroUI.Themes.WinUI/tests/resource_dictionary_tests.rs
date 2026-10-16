//! The rule of `tests/Themes.UnitTests/ResourceDictionaryTests.cs` of the upstream project (as
//! the tests of the Fluent theme), for the three theme dictionaries of the WinUI theme.

use super::support::*;
use crate::WinUITheme;
use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::styling::ThemeVariant;

#[test]
fn should_define_identical_keys_in_theme_dictionaries() {
    // Any resource defined in one ThemeDictionary, must be defined in the rest
    let _app = start_application();
    let theme = create_attached_theme();

    // The theme defines its theme dictionaries on the root style object: the resource documents
    // are merged into its dictionary.
    let flat: Vec<(ThemeVariant, Vec<ResourceKey>)> = theme
        .resources()
        .theme_dictionaries_snapshot()
        .into_iter()
        .map(|(variant, provider)| {
            let dictionary = provider
                .as_object()
                .and_then(|object| object.downcast_ref::<ResourceDictionary>())
                .expect("a resource dictionary")
                .to_ref();
            let mut entries = Vec::new();
            enumerate_dictionary_resources(&dictionary, None, &mut entries);
            (variant, entries.into_iter().map(|entry| entry.key).collect())
        })
        .collect();

    // Default (the dark theme of WinUI), Light and HighContrast.
    assert_eq!(3, flat.len());
    assert!(flat.iter().any(|(variant, _)| *variant == ThemeVariant::default()));
    assert!(flat.iter().any(|(variant, _)| *variant == ThemeVariant::light()));
    assert!(flat.iter().any(|(variant, _)| *variant == WinUITheme::high_contrast()));
    for (variant, keys) in &flat {
        let other_keys: Vec<&ResourceKey> =
            flat.iter().filter(|(other, _)| other != variant).flat_map(|(_, keys)| keys.iter()).collect();
        for key in keys {
            assert!(other_keys.contains(&key), "{key:?} of {variant:?} is not defined in the other variants");
        }
    }
}
