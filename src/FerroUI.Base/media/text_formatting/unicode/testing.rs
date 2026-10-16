//! The generated tries of the Unicode data, for the benchmarks of the
//! framework: upstream's benchmark project reads the internal tries
//! (`UnicodeDataTrie.Trie`, `BiDiTrie.Trie`, `SegmentationTrie.Trie`,
//! `EastAsianWidthTrie.Trie`) through its access to the internals of the
//! framework, and this module is that access.

/// A read only view over one of the generated tries (`UnicodeTrie`).
#[derive(Clone, Copy, Debug)]
pub struct UnicodeTrie(super::UnicodeTrie<'static>);

impl UnicodeTrie {
    /// `UnicodeTrie.Get`: the value stored for a code point.
    #[inline(always)]
    pub fn get(&self, code_point: u32) -> u32 {
        self.0.get(code_point)
    }
}

/// `UnicodeDataTrie.Trie`: the general category, the script and the script
/// extensions, packed.
#[inline(always)]
pub fn unicode_data_trie() -> UnicodeTrie {
    UnicodeTrie(super::UnicodeDataTrie::trie())
}

/// `BiDiTrie.Trie`: the bidirectional class and the paired bracket data.
#[inline(always)]
pub fn bi_di_trie() -> UnicodeTrie {
    UnicodeTrie(super::BiDiTrie::trie())
}

/// `SegmentationTrie.Trie`: the grapheme, line, word and sentence break classes.
#[inline(always)]
pub fn segmentation_trie() -> UnicodeTrie {
    UnicodeTrie(super::SegmentationTrie::trie())
}

/// `EastAsianWidthTrie.Trie`: the East Asian width class.
#[inline(always)]
pub fn east_asian_width_trie() -> UnicodeTrie {
    UnicodeTrie(super::EastAsianWidthTrie::trie())
}
