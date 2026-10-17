use super::{Block, Inline, LineStackingStrategy, Paragraph, TextElement};
use ferroui_base::media::{
    BaselineAlignment, FlowDirection, FontFamily, FontFeatureCollection, FontStretch, FontStyle,
    FontWeight, IBrush, MediaCollection, TextAlignment, TextDecoration, TextDecorationCollection,
};
use ferroui_base::{Ref, Thickness};
use std::rc::Rc;

/// A character-format patch, also used to capture the assigned formatting of an
/// element. `None` leaves a property unchanged. For nullable properties,
/// `Some(None)` explicitly clears it. Snapshots copy mutable formatting collections.
#[derive(Default)]
pub struct CharacterFormat {
    pub font_family: Option<FontFamily>,
    pub font_size: Option<f64>,
    pub font_style: Option<FontStyle>,
    pub font_weight: Option<FontWeight>,
    pub font_stretch: Option<FontStretch>,
    pub font_features: Option<Option<FontFeatureCollection>>,
    pub foreground: Option<Option<Rc<dyn IBrush>>>,
    pub background: Option<Option<Rc<dyn IBrush>>>,
    pub text_decorations: Option<Option<TextDecorationCollection>>,
    pub baseline_alignment: Option<BaselineAlignment>,
    pub letter_spacing: Option<f64>,
}

impl std::fmt::Debug for CharacterFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CharacterFormat")
            .field("font_family", &self.font_family)
            .field("font_size", &self.font_size)
            .field("font_style", &self.font_style)
            .field("font_weight", &self.font_weight)
            .field("font_stretch", &self.font_stretch)
            .field("foreground", &self.foreground)
            .field("background", &self.background)
            .field("baseline_alignment", &self.baseline_alignment)
            .field("letter_spacing", &self.letter_spacing)
            .field(
                "font_features",
                &self
                    .font_features
                    .as_ref()
                    .map(|v| v.as_ref().map(|v| v.list().snapshot())),
            )
            .field(
                "text_decorations",
                &self
                    .text_decorations
                    .as_ref()
                    .map(|v| v.as_ref().map(|v| v.list().snapshot())),
            )
            .finish()
    }
}

impl Clone for CharacterFormat {
    fn clone(&self) -> Self {
        Self {
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_style: self.font_style,
            font_weight: self.font_weight,
            font_stretch: self.font_stretch,
            font_features: self
                .font_features
                .as_ref()
                .map(|v| v.as_ref().map(copy_features)),
            foreground: self
                .foreground
                .as_ref()
                .map(|v| v.as_ref().map(freeze_brush)),
            background: self
                .background
                .as_ref()
                .map(|v| v.as_ref().map(freeze_brush)),
            text_decorations: self
                .text_decorations
                .as_ref()
                .map(|v| v.as_ref().map(copy_decorations)),
            baseline_alignment: self.baseline_alignment,
            letter_spacing: self.letter_spacing,
        }
    }
}

impl PartialEq for CharacterFormat {
    fn eq(&self, other: &Self) -> bool {
        self.font_family == other.font_family
            && self.font_size == other.font_size
            && self.font_style == other.font_style
            && self.font_weight == other.font_weight
            && self.font_stretch == other.font_stretch
            && self.foreground == other.foreground
            && self.background == other.background
            && self.baseline_alignment == other.baseline_alignment
            && self.letter_spacing == other.letter_spacing
            && nested_eq(&self.font_features, &other.font_features, |a, b| {
                a.list().snapshot() == b.list().snapshot()
            })
            && nested_eq(
                &self.text_decorations,
                &other.text_decorations,
                decorations_equal,
            )
    }
}

fn nested_eq<T>(
    a: &Option<Option<T>>,
    b: &Option<Option<T>>,
    compare: impl Fn(&T, &T) -> bool,
) -> bool {
    match (a, b) {
        (None, None) | (Some(None), Some(None)) => true,
        (Some(Some(a)), Some(Some(b))) => compare(a, b),
        _ => false,
    }
}

impl CharacterFormat {
    /// Captures assigned properties without flattening inherited span formatting.
    pub fn from_element(element: &TextElement) -> Self {
        Self {
            font_family: element.get_base_value(TextElement::font_family_property()),
            font_size: element.get_base_value(TextElement::font_size_property()),
            font_style: element.get_base_value(TextElement::font_style_property()),
            font_weight: element.get_base_value(TextElement::font_weight_property()),
            font_stretch: element.get_base_value(TextElement::font_stretch_property()),
            font_features: element
                .get_base_value(TextElement::font_features_property())
                .map(|v| v.as_ref().map(copy_features)),
            foreground: element
                .get_base_value(TextElement::foreground_property())
                .map(|v| v.as_ref().map(freeze_brush)),
            background: element
                .get_base_value(TextElement::background_property())
                .map(|v| v.as_ref().map(freeze_brush)),
            text_decorations: element
                .get_base_value(Inline::text_decorations_property())
                .map(|v| v.as_ref().map(copy_decorations)),
            baseline_alignment: element.get_base_value(Inline::baseline_alignment_property()),
            letter_spacing: element.get_base_value(TextElement::letter_spacing_property()),
        }
    }

    /// Captures computed values, including formatting inherited through logical parents.
    pub fn effective(element: &TextElement) -> Self {
        Self {
            font_family: Some(element.font_family()),
            font_size: Some(element.font_size()),
            font_style: Some(element.font_style()),
            font_weight: Some(element.font_weight()),
            font_stretch: Some(element.font_stretch()),
            font_features: Some(element.font_features().as_ref().map(copy_features)),
            foreground: Some(element.foreground().as_ref().map(freeze_brush)),
            background: Some(element.background().as_ref().map(freeze_brush)),
            text_decorations: Some(
                element
                    .get_value(Inline::text_decorations_property())
                    .as_ref()
                    .map(copy_decorations),
            ),
            baseline_alignment: Some(element.get_value(Inline::baseline_alignment_property())),
            letter_spacing: Some(element.letter_spacing()),
        }
    }

    /// Returns this format with every assigned field of `patch` replaced.
    pub fn overlay(&self, patch: &Self) -> Self {
        let mut result = self.clone();
        macro_rules! overlay {
            ($($field:ident),* $(,)?) => { $(if patch.$field.is_some() { result.$field = patch.$field.clone(); })* };
        }
        overlay!(
            font_family,
            font_size,
            font_style,
            font_weight,
            font_stretch,
            font_features,
            foreground,
            background,
            text_decorations,
            baseline_alignment,
            letter_spacing
        );
        result
    }

    pub fn validate(&self) -> Result<(), super::DocumentError> {
        if self.font_size.is_some_and(|v| !v.is_finite() || v <= 0.0) {
            return Err(super::DocumentError::InvalidFormat(
                "FontSize must be finite and positive",
            ));
        }
        if self.letter_spacing.is_some_and(|v| !v.is_finite()) {
            return Err(super::DocumentError::InvalidFormat(
                "LetterSpacing must be finite",
            ));
        }
        Ok(())
    }

    pub fn try_apply_to(&self, element: &TextElement) -> Result<(), super::DocumentError> {
        self.validate()?;
        self.apply_to(element);
        Ok(())
    }

    pub(crate) fn apply_to(&self, element: &TextElement) {
        if let Some(value) = &self.font_family {
            element.set_font_family(value.clone());
        }
        if let Some(value) = self.font_size {
            element.set_font_size(value);
        }
        if let Some(value) = self.font_style {
            element.set_font_style(value);
        }
        if let Some(value) = self.font_weight {
            element.set_font_weight(value);
        }
        if let Some(value) = self.font_stretch {
            element.set_font_stretch(value);
        }
        if let Some(value) = &self.font_features {
            element.set_font_features(value.as_ref().map(copy_features));
        }
        if let Some(value) = &self.foreground {
            element.set_foreground(value.as_ref().map(freeze_brush));
        }
        if let Some(value) = &self.background {
            element.set_background(value.as_ref().map(freeze_brush));
        }
        if let Some(value) = &self.text_decorations {
            element.set_value(
                Inline::text_decorations_property(),
                value.as_ref().map(copy_decorations),
            );
        }
        if let Some(value) = self.baseline_alignment {
            element.set_value(Inline::baseline_alignment_property(), value);
        }
        if let Some(value) = self.letter_spacing {
            element.set_letter_spacing(value);
        }
    }

    pub(crate) fn replace_on(&self, element: &TextElement) {
        element.clear_value(TextElement::font_family_property());
        element.clear_value(TextElement::font_size_property());
        element.clear_value(TextElement::font_style_property());
        element.clear_value(TextElement::font_weight_property());
        element.clear_value(TextElement::font_stretch_property());
        element.clear_value(TextElement::font_features_property());
        element.clear_value(TextElement::foreground_property());
        element.clear_value(TextElement::background_property());
        element.clear_value(Inline::text_decorations_property());
        element.clear_value(Inline::baseline_alignment_property());
        element.clear_value(TextElement::letter_spacing_property());
        self.apply_to(element);
    }
}

/// Paragraph formatting and inherited character defaults, represented as a patch.
#[derive(Clone, Debug, Default)]
pub struct ParagraphFormat {
    pub character: CharacterFormat,
    pub text_alignment: Option<TextAlignment>,
    pub line_height: Option<f64>,
    pub line_stacking_strategy: Option<LineStackingStrategy>,
    pub text_indent: Option<f64>,
    pub margin: Option<Thickness>,
    pub flow_direction: Option<FlowDirection>,
}

impl PartialEq for ParagraphFormat {
    fn eq(&self, other: &Self) -> bool {
        self.character == other.character
            && self.text_alignment == other.text_alignment
            && option_float_equal(self.line_height, other.line_height)
            && self.line_stacking_strategy == other.line_stacking_strategy
            && self.text_indent == other.text_indent
            && self.margin == other.margin
            && self.flow_direction == other.flow_direction
    }
}

impl ParagraphFormat {
    pub fn from_paragraph(paragraph: &Paragraph) -> Self {
        Self {
            character: CharacterFormat::from_element(paragraph),
            text_alignment: paragraph.get_base_value(Block::text_alignment_property()),
            line_height: paragraph.get_base_value(Block::line_height_property()),
            line_stacking_strategy: paragraph
                .get_base_value(Block::line_stacking_strategy_property()),
            text_indent: paragraph.get_base_value(Paragraph::text_indent_property()),
            margin: paragraph.get_base_value(Block::margin_property()),
            flow_direction: paragraph.get_base_value(Block::flow_direction_property()),
        }
    }

    pub fn validate(&self) -> Result<(), super::DocumentError> {
        self.character.validate()?;
        if self
            .line_height
            .is_some_and(|v| (!v.is_finite() && !v.is_nan()) || v < 0.0)
        {
            return Err(super::DocumentError::InvalidFormat(
                "LineHeight must be natural or nonnegative",
            ));
        }
        if self.text_indent.is_some_and(|v| !v.is_finite()) {
            return Err(super::DocumentError::InvalidFormat(
                "TextIndent must be finite",
            ));
        }
        if self.margin.is_some_and(|v| {
            [v.left, v.top, v.right, v.bottom]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0)
        }) {
            return Err(super::DocumentError::InvalidFormat(
                "Margin must be finite and nonnegative",
            ));
        }
        Ok(())
    }

    pub fn overlay(&self, patch: &Self) -> Self {
        let mut result = self.clone();
        result.character = result.character.overlay(&patch.character);
        macro_rules! overlay {
            ($($field:ident),* $(,)?) => { $(if patch.$field.is_some() { result.$field = patch.$field; })* };
        }
        overlay!(
            text_alignment,
            line_height,
            line_stacking_strategy,
            text_indent,
            margin,
            flow_direction
        );
        result
    }

    pub fn try_apply_to(&self, paragraph: &Paragraph) -> Result<(), super::DocumentError> {
        self.validate()?;
        self.apply_to(paragraph);
        Ok(())
    }

    pub(crate) fn apply_to(&self, paragraph: &Paragraph) {
        self.character.apply_to(paragraph);
        if let Some(value) = self.text_alignment {
            paragraph.set_text_alignment(value);
        }
        if let Some(value) = self.line_height {
            paragraph.set_line_height(value);
        }
        if let Some(value) = self.line_stacking_strategy {
            paragraph.set_line_stacking_strategy(value);
        }
        if let Some(value) = self.text_indent {
            paragraph.set_text_indent(value);
        }
        if let Some(value) = self.margin {
            paragraph.set_margin(value);
        }
        if let Some(value) = self.flow_direction {
            paragraph.set_flow_direction(value);
        }
    }

    pub(crate) fn replace_on(&self, paragraph: &Paragraph) {
        self.character.replace_on(paragraph);
        paragraph.clear_value(Block::text_alignment_property());
        paragraph.clear_value(Block::line_height_property());
        paragraph.clear_value(Block::line_stacking_strategy_property());
        paragraph.clear_value(Paragraph::text_indent_property());
        paragraph.clear_value(Block::margin_property());
        paragraph.clear_value(Block::flow_direction_property());
        self.apply_to(paragraph);
    }
}

fn option_float_equal(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b || (a.is_nan() && b.is_nan()),
        (None, None) => true,
        _ => false,
    }
}

fn copy_features(value: &FontFeatureCollection) -> FontFeatureCollection {
    FontFeatureCollection::from_items(value.list().snapshot().iter().cloned())
}

// Framework brushes with a value snapshot are frozen. Scene/custom brushes are
// service objects and retain their identity, just as embedded controls do.
fn freeze_brush(value: &Rc<dyn IBrush>) -> Rc<dyn IBrush> {
    if (value.as_solid_color_brush().is_some()
        || value.as_image_brush().is_some()
        || value.as_gradient_brush().is_some())
        && value.as_mutable_brush().is_some()
    {
        if let Some(mutable) = value.as_mutable_brush() {
            return mutable.to_immutable();
        }
    }
    value.clone()
}

fn copy_decorations(value: &TextDecorationCollection) -> TextDecorationCollection {
    TextDecorationCollection::from_items(value.list().snapshot().iter().map(copy_decoration))
}

fn copy_decoration(value: &Ref<TextDecoration>) -> Ref<TextDecoration> {
    let result = TextDecoration::new();
    result.set_location(value.location());
    result.set_stroke(value.stroke().as_ref().map(freeze_brush));
    result.set_stroke_thickness_unit(value.stroke_thickness_unit());
    result.set_stroke_dash_array(
        value
            .stroke_dash_array()
            .map(|v| MediaCollection::from_items(v.list().snapshot().iter().copied())),
    );
    result.set_stroke_dash_offset(value.stroke_dash_offset());
    result.set_stroke_thickness(value.stroke_thickness());
    result.set_stroke_line_cap(value.stroke_line_cap());
    result.set_stroke_offset(value.stroke_offset());
    result.set_stroke_offset_unit(value.stroke_offset_unit());
    result
}

fn decorations_equal(a: &TextDecorationCollection, b: &TextDecorationCollection) -> bool {
    let a = a.list().snapshot();
    let b = b.list().snapshot();
    a.len() == b.len()
        && a.iter().zip(b.iter()).all(|(a, b)| {
            a.location() == b.location()
                && a.stroke() == b.stroke()
                && a.stroke_thickness_unit() == b.stroke_thickness_unit()
                && a.stroke_dash_array().map(|v| v.list().snapshot())
                    == b.stroke_dash_array().map(|v| v.list().snapshot())
                && a.stroke_dash_offset() == b.stroke_dash_offset()
                && a.stroke_thickness() == b.stroke_thickness()
                && a.stroke_line_cap() == b.stroke_line_cap()
                && a.stroke_offset() == b.stroke_offset()
                && a.stroke_offset_unit() == b.stroke_offset_unit()
        })
}
