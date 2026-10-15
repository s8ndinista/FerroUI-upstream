//! Port of `Vnc/VncEncoding.cs` of RemoteViewing (BSD-2-Clause, Copyright
//! (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).
//!
//! Several names stand for one value, and a client may name encodings the
//! list does not know, so the encodings are constants of a transparent
//! wrapper of the value and not the variants of an enum.

use std::fmt;

/// Specifies the VNC encoding used for a framebuffer update rectangle, or a
/// pseudo-encoding a client announces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VncEncoding(pub i32);

#[allow(non_upper_case_globals)]
impl VncEncoding {
    /// Raw encoding.
    pub const Raw: VncEncoding = VncEncoding(0);
    /// Copy rectangle encoding.
    pub const CopyRect: VncEncoding = VncEncoding(1);
    /// Rise-and-run-length encoding.
    pub const RRE: VncEncoding = VncEncoding(2);
    /// Compact RRE encoding.
    pub const CoRRE: VncEncoding = VncEncoding(4);
    /// Hextile encoding.
    pub const Hextile: VncEncoding = VncEncoding(5);
    /// Zlib encoding.
    pub const Zlib: VncEncoding = VncEncoding(6);
    /// Tight encoding.
    pub const Tight: VncEncoding = VncEncoding(7);
    /// Zlib-compressed Hextile encoding.
    pub const ZlibHex: VncEncoding = VncEncoding(8);
    /// Tiled run-length encoding.
    pub const TRLE: VncEncoding = VncEncoding(15);
    /// Zlib run-length encoding.
    pub const Zrle: VncEncoding = VncEncoding(16);
    /// JPEG encoding.
    pub const Jpeg: VncEncoding = VncEncoding(21);
    /// JPEG run-length encoding.
    pub const JRLE: VncEncoding = VncEncoding(22);
    /// Tight PNG encoding.
    pub const TightPng: VncEncoding = VncEncoding(-260);
    /// The fence pseudo-encoding.
    pub const Fence: VncEncoding = VncEncoding(-312);
    /// The continuous updates pseudo-encoding.
    pub const ContinuousUpdates: VncEncoding = VncEncoding(-313);
    /// The cursor with alpha pseudo-encoding.
    pub const CursorWithAlpha: VncEncoding = VncEncoding(-314);
    /// The cursor pseudo-encoding.
    pub const PseudoCursor: VncEncoding = VncEncoding(-239);
    /// The desktop size pseudo-encoding.
    pub const PseudoDesktopSize: VncEncoding = VncEncoding(-223);
    /// The X cursor pseudo-encoding.
    pub const XCursor: VncEncoding = VncEncoding(-240);
    /// The rich cursor pseudo-encoding.
    pub const RichCursor: VncEncoding = VncEncoding(-239);
    /// The pointer position pseudo-encoding.
    pub const PointerPox: VncEncoding = VncEncoding(-232);
    /// The last rectangle pseudo-encoding.
    pub const LastRect: VncEncoding = VncEncoding(-224);
    /// The desktop size pseudo-encoding.
    pub const DesktopSize: VncEncoding = VncEncoding(-223);
    /// Tight compression level 9.
    pub const TightCompressionLevel9: VncEncoding = VncEncoding(-247);
    /// Tight compression level 8.
    pub const TightCompressionLevel8: VncEncoding = VncEncoding(-248);
    /// Tight compression level 7.
    pub const TightCompressionLevel7: VncEncoding = VncEncoding(-249);
    /// Tight compression level 6.
    pub const TightCompressionLevel6: VncEncoding = VncEncoding(-250);
    /// Tight compression level 5.
    pub const TightCompressionLevel5: VncEncoding = VncEncoding(-251);
    /// Tight compression level 4.
    pub const TightCompressionLevel4: VncEncoding = VncEncoding(-252);
    /// Tight compression level 3.
    pub const TightCompressionLevel3: VncEncoding = VncEncoding(-253);
    /// Tight compression level 2.
    pub const TightCompressionLevel2: VncEncoding = VncEncoding(-254);
    /// Tight compression level 1.
    pub const TightCompressionLevel1: VncEncoding = VncEncoding(-255);
    /// Tight compression level 0.
    pub const TightCompressionLevel0: VncEncoding = VncEncoding(-256);
    /// Tight quality level 9.
    pub const TightQualityLevel9: VncEncoding = VncEncoding(-23);
    /// Tight quality level 8.
    pub const TightQualityLevel8: VncEncoding = VncEncoding(-24);
    /// Tight quality level 7.
    pub const TightQualityLevel7: VncEncoding = VncEncoding(-25);
    /// Tight quality level 6.
    pub const TightQualityLevel6: VncEncoding = VncEncoding(-26);
    /// Tight quality level 5.
    pub const TightQualityLevel5: VncEncoding = VncEncoding(-27);
    /// Tight quality level 4.
    pub const TightQualityLevel4: VncEncoding = VncEncoding(-28);
    /// Tight quality level 3.
    pub const TightQualityLevel3: VncEncoding = VncEncoding(-29);
    /// Tight quality level 2.
    pub const TightQualityLevel2: VncEncoding = VncEncoding(-30);
    /// Tight quality level 1.
    pub const TightQualityLevel1: VncEncoding = VncEncoding(-31);
    /// Tight quality level 0.
    pub const TightQualityLevel0: VncEncoding = VncEncoding(-32);
    /// VMware pseudo-encoding.
    pub const VMWd: VncEncoding = VncEncoding(0x574d5664);
    /// VMware pseudo-encoding.
    pub const VMWe: VncEncoding = VncEncoding(0x574d5665);
    /// VMware pseudo-encoding.
    pub const VMWf: VncEncoding = VncEncoding(0x574d5666);
    /// VMware pseudo-encoding.
    pub const VMWg: VncEncoding = VncEncoding(0x574d5667);
    /// VMware pseudo-encoding.
    pub const VMWh: VncEncoding = VncEncoding(0x574d5668);
    /// VMware pseudo-encoding.
    pub const VMWi: VncEncoding = VncEncoding(0x574d5669);
    /// VMware pseudo-encoding.
    pub const VMWj: VncEncoding = VncEncoding(0x574d566a);
    /// The QEMU pointer motion change pseudo-encoding.
    pub const QemuPointerMotionChange: VncEncoding = VncEncoding(-257);
    /// The QEMU extended key event pseudo-encoding.
    pub const QemuExtendedKeyEvent: VncEncoding = VncEncoding(-258);
    /// The QEMU audio pseudo-encoding.
    pub const QemuAudio: VncEncoding = VncEncoding(-259);
    /// The LED state pseudo-encoding.
    pub const LedState: VncEncoding = VncEncoding(-261);
    /// The general input interface pseudo-encoding.
    pub const Gii: VncEncoding = VncEncoding(-305);
    /// The desktop name pseudo-encoding.
    pub const DesktopName: VncEncoding = VncEncoding(-307);
    /// The extended desktop size pseudo-encoding.
    pub const ExtendedDesktopSize: VncEncoding = VncEncoding(-308);
    /// The XVP pseudo-encoding.
    pub const Xvp: VncEncoding = VncEncoding(-309);
    /// The continuous update pseudo-encoding.
    pub const ContinuousUpdate: VncEncoding = VncEncoding(-313);

    /// The names of the values, in the order of the original: the name a
    /// value is shown with is the first that has it.
    const NAMES: &'static [(VncEncoding, &'static str)] = &[
        (Self::Raw, "Raw"),
        (Self::CopyRect, "CopyRect"),
        (Self::RRE, "RRE"),
        (Self::CoRRE, "CoRRE"),
        (Self::Hextile, "Hextile"),
        (Self::Zlib, "Zlib"),
        (Self::Tight, "Tight"),
        (Self::ZlibHex, "ZlibHex"),
        (Self::TRLE, "TRLE"),
        (Self::Zrle, "Zrle"),
        (Self::Jpeg, "Jpeg"),
        (Self::JRLE, "JRLE"),
        (Self::TightPng, "TightPng"),
        (Self::Fence, "Fence"),
        (Self::ContinuousUpdates, "ContinuousUpdates"),
        (Self::CursorWithAlpha, "CursorWithAlpha"),
        (Self::PseudoCursor, "PseudoCursor"),
        (Self::PseudoDesktopSize, "PseudoDesktopSize"),
        (Self::XCursor, "XCursor"),
        (Self::PointerPox, "PointerPox"),
        (Self::LastRect, "LastRect"),
        (Self::TightCompressionLevel9, "TightCompressionLevel9"),
        (Self::TightCompressionLevel8, "TightCompressionLevel8"),
        (Self::TightCompressionLevel7, "TightCompressionLevel7"),
        (Self::TightCompressionLevel6, "TightCompressionLevel6"),
        (Self::TightCompressionLevel5, "TightCompressionLevel5"),
        (Self::TightCompressionLevel4, "TightCompressionLevel4"),
        (Self::TightCompressionLevel3, "TightCompressionLevel3"),
        (Self::TightCompressionLevel2, "TightCompressionLevel2"),
        (Self::TightCompressionLevel1, "TightCompressionLevel1"),
        (Self::TightCompressionLevel0, "TightCompressionLevel0"),
        (Self::TightQualityLevel9, "TightQualityLevel9"),
        (Self::TightQualityLevel8, "TightQualityLevel8"),
        (Self::TightQualityLevel7, "TightQualityLevel7"),
        (Self::TightQualityLevel6, "TightQualityLevel6"),
        (Self::TightQualityLevel5, "TightQualityLevel5"),
        (Self::TightQualityLevel4, "TightQualityLevel4"),
        (Self::TightQualityLevel3, "TightQualityLevel3"),
        (Self::TightQualityLevel2, "TightQualityLevel2"),
        (Self::TightQualityLevel1, "TightQualityLevel1"),
        (Self::TightQualityLevel0, "TightQualityLevel0"),
        (Self::VMWd, "VMWd"),
        (Self::VMWe, "VMWe"),
        (Self::VMWf, "VMWf"),
        (Self::VMWg, "VMWg"),
        (Self::VMWh, "VMWh"),
        (Self::VMWi, "VMWi"),
        (Self::VMWj, "VMWj"),
        (Self::QemuPointerMotionChange, "QemuPointerMotionChange"),
        (Self::QemuExtendedKeyEvent, "QemuExtendedKeyEvent"),
        (Self::QemuAudio, "QemuAudio"),
        (Self::LedState, "LedState"),
        (Self::Gii, "Gii"),
        (Self::DesktopName, "DesktopName"),
        (Self::ExtendedDesktopSize, "ExtendedDesktopSize"),
        (Self::Xvp, "Xvp"),
    ];
}

impl fmt::Display for VncEncoding {
    /// The name of the encoding, or its number when it has none (as an enum
    /// value of .NET is shown).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match Self::NAMES.iter().find(|(encoding, _)| encoding == self) {
            Some((_, name)) => f.write_str(name),
            None => write!(f, "{}", self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn an_encoding_shows_its_name_or_its_number() {
        assert_eq!("Tight", VncEncoding::Tight.to_string());
        assert_eq!("ExtendedDesktopSize", VncEncoding(-308).to_string());
        assert_eq!("PseudoDesktopSize", VncEncoding::DesktopSize.to_string());
        assert_eq!("12345", VncEncoding(12345).to_string());
    }
}
