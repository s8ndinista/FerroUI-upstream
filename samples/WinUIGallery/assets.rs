//! Original pinned control-detail images exposed through the native asset loader.

use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::{Uri, UriKind};
use std::sync::Once;

pub(crate) fn register() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        ferroui_base::platform::register_assets("WinUIGallery", crate::assets_data::ASSETS);
    });
}

pub(crate) fn load_bitmap(uri: &str) -> std::io::Result<Bitmap> {
    register();
    let uri = Uri::new(uri, UriKind::Absolute).map_err(|error| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string())
    })?;
    let mut stream = AssetLoader::open(&uri, None)?;
    Bitmap::from_stream(&mut *stream)
}
