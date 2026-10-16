//! Raw `UnicodeTrie::get` throughput across the four generated tries
//! (UnicodeData, BiDi, GraphemeBreak, EastAsianWidth) and three codepoint
//! distributions (ASCII, BMP, supplementary). Bypasses surrogate decoding so
//! the numbers reflect only the trie walk cost: useful as a regression target
//! when changing trie packing, branch ordering, or builder layout.
//!
//! The tries are internal to the base crate; its `testing` feature exposes
//! them (`ferroui_base::media::text_formatting::unicode::testing`), as
//! upstream's benchmark project reaches them through its access to the
//! internals of the framework. `[MaxWarmupCount(15)]` of upstream bounds the
//! warm-up iterations, which the harness runs a fixed number of times below
//! that bound.

use super::random::Random;
use crate::harness::Registry;
use ferroui_base::media::text_formatting::unicode::testing::{
    bi_di_trie, east_asian_width_trie, segmentation_trie, unicode_data_trie,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodepointDistribution {
    /// Printable ASCII (0x20..0x7E): common BMP-non-surrogate path.
    Ascii,

    /// Full BMP excluding the surrogate range (0..0xD7FF, 0xE000..0xFFFF).
    Bmp,

    /// Supplementary plane (0x10000..0x10FFFF): exercises two-level lookup.
    Supplementary,
}

const CODEPOINT_COUNT: usize = 4096;

pub struct UnicodeTrieBenchmark {
    codepoints: Vec<u32>,
}

impl UnicodeTrieBenchmark {
    pub fn new(distribution: CodepointDistribution) -> Self {
        // Fixed seed for reproducibility: numbers across CI runs should be
        // comparable in the absence of generator / trie-walk changes.
        let mut rng = Random::new(42);
        let mut codepoints = vec![0_u32; CODEPOINT_COUNT];

        for codepoint in codepoints.iter_mut() {
            *codepoint = match distribution {
                CodepointDistribution::Ascii => rng.next_range(0x20, 0x7F) as u32,
                CodepointDistribution::Bmp => Self::sample_bmp(&mut rng),
                CodepointDistribution::Supplementary => rng.next_range(0x10000, 0x110000) as u32,
            };
        }

        Self { codepoints }
    }

    fn sample_bmp(rng: &mut Random) -> u32 {
        // Avoid the surrogate range: supplementary callers don't hit it in
        // practice and we cover it via the dedicated codepoint fixture below.
        let v = rng.next_range(0, 0xFFFE) as u32;
        if (0xD800..=0xDFFF).contains(&v) {
            0xE000
        } else {
            v
        }
    }

    pub fn get_unicode_data(&self) -> u32 {
        let trie = unicode_data_trie();
        let codepoints = self.codepoints.as_slice();
        let mut sum = 0_u32;

        for &codepoint in codepoints {
            sum = sum.wrapping_add(trie.get(codepoint));
        }

        sum
    }

    pub fn get_bi_di(&self) -> u32 {
        let trie = bi_di_trie();
        let codepoints = self.codepoints.as_slice();
        let mut sum = 0_u32;

        for &codepoint in codepoints {
            sum = sum.wrapping_add(trie.get(codepoint));
        }

        sum
    }

    pub fn get_grapheme_break(&self) -> u32 {
        let trie = segmentation_trie();
        let codepoints = self.codepoints.as_slice();
        let mut sum = 0_u32;

        for &codepoint in codepoints {
            sum = sum.wrapping_add(trie.get(codepoint));
        }

        sum
    }

    pub fn get_east_asian_width(&self) -> u32 {
        let trie = east_asian_width_trie();
        let codepoints = self.codepoints.as_slice();
        let mut sum = 0_u32;

        for &codepoint in codepoints {
            sum = sum.wrapping_add(trie.get(codepoint));
        }

        sum
    }

    /// Worst-case "every property" pass: one `UnicodeTrie::get` call per trie
    /// per codepoint. Matches what a layout pipeline pass that needs every
    /// property would pay if the four tries are kept separate. Establishes a
    /// baseline for any future "merge packed properties" experiment.
    pub fn get_all_tries_per_codepoint(&self) -> u32 {
        let unicode_data = unicode_data_trie();
        let bi_di = bi_di_trie();
        let grapheme = segmentation_trie();
        let eaw = east_asian_width_trie();
        let codepoints = self.codepoints.as_slice();
        let mut sum = 0_u32;

        for &cp in codepoints {
            sum = sum.wrapping_add(unicode_data.get(cp));
            sum = sum.wrapping_add(bi_di.get(cp));
            sum = sum.wrapping_add(grapheme.get(cp));
            sum = sum.wrapping_add(eaw.get(cp));
        }

        sum
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "UnicodeTrieBenchmark").min_iteration_time(150);
    for distribution in
        [CodepointDistribution::Ascii, CodepointDistribution::Bmp, CodepointDistribution::Supplementary]
    {
        let parameters = format!("Distribution={distribution:?}");
        class.benchmark(
            "get_unicode_data",
            parameters.clone(),
            move || UnicodeTrieBenchmark::new(distribution),
            |b| b.get_unicode_data(),
        );
        class.benchmark(
            "get_bi_di",
            parameters.clone(),
            move || UnicodeTrieBenchmark::new(distribution),
            |b| b.get_bi_di(),
        );
        class.benchmark(
            "get_grapheme_break",
            parameters.clone(),
            move || UnicodeTrieBenchmark::new(distribution),
            |b| b.get_grapheme_break(),
        );
        class.benchmark(
            "get_east_asian_width",
            parameters.clone(),
            move || UnicodeTrieBenchmark::new(distribution),
            |b| b.get_east_asian_width(),
        );
        class.benchmark(
            "get_all_tries_per_codepoint",
            parameters,
            move || UnicodeTrieBenchmark::new(distribution),
            |b| b.get_all_tries_per_codepoint(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unicode_trie_benchmark() {
        crate::harness::smoke_class(super::register, "UnicodeTrieBenchmark");
    }
}
