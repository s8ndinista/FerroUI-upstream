//! Default palettes from the pinned WinUI InkToolbarPenButton and HighlighterButton sources.
use ferroui_base::collections::FerroList;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use std::rc::Rc;

const PEN_COLORS: [u32; 30] = [
    0xff000000, 0xffffffff, 0xffd1d3d4, 0xffa7a9ac, 0xff808285, 0xff58595b, 0xffb31564, 0xffe61b1b,
    0xffff5500, 0xffffaa00, 0xffffce00, 0xffffe600, 0xffa2e61b, 0xff26e600, 0xff008055, 0xff00aacc,
    0xff004de6, 0xff3d00b8, 0xff6600cc, 0xff600080, 0xfff7d7c4, 0xffbb9167, 0xff8e562e, 0xff613d30,
    0xffff80ff, 0xffffc680, 0xffffff80, 0xff80ff9e, 0xff80d6ff, 0xffbcb3ff,
];
const HIGHLIGHTER_COLORS: [u32; 6] = [
    0xffffe600, 0xff26e600, 0xff44c8f5, 0xffec008c, 0xffff5500, 0xff6600cc,
];

pub(crate) fn default_palette(highlighter: bool) -> FerroList<Rc<dyn IBrush>> {
    let colors: &[u32] = if highlighter {
        &HIGHLIGHTER_COLORS
    } else {
        &PEN_COLORS
    };
    FerroList::from_items(colors.iter().map(|&value| {
        Rc::new(ImmutableSolidColorBrush::new(Color::from_uint32(value))) as Rc<dyn IBrush>
    }))
}

/// English fallback names; application localization may override the automation properties.
pub(crate) fn color_name(color: Color) -> String {
    let value = color.to_uint32();
    let name = match value {
        0xff000000 => "Black",
        0xffffffff => "White",
        0xffd1d3d4 => "Light gray",
        0xffa7a9ac => "Gray",
        0xff808285 => "Medium gray",
        0xff58595b => "Dark gray",
        0xffb31564 => "Magenta",
        0xffe61b1b => "Red",
        0xffff5500 => "Orange",
        0xffffaa00 => "Amber",
        0xffffce00 => "Gold",
        0xffffe600 => "Yellow",
        0xffa2e61b => "Lime",
        0xff26e600 => "Green",
        0xff008055 => "Dark green",
        0xff00aacc => "Cyan",
        0xff004de6 => "Blue",
        0xff3d00b8 => "Indigo",
        0xff6600cc => "Purple",
        0xff600080 => "Dark purple",
        0xff44c8f5 => "Light blue",
        0xffec008c => "Pink",
        _ => return format!("#{value:08X}"),
    };
    name.to_owned()
}
