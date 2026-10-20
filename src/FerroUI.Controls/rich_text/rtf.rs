use crate::documents::{
    CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, LineSpacingRule,
    LineStackingStrategy, ParagraphFormat, ParagraphSnapshot, TabAlignment, TabLeader, TextTabStop,
};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    BaselineAlignment, Color, FlowDirection, FontFamily, FontStretch, FontStyle, FontWeight,
    IBrush, TextAlignment, TextDecorationCollection, TextDecorationLocation, TextDecorations,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::Thickness;
use std::collections::BTreeMap;
use std::fmt::{self, Write};
use std::rc::Rc;

/// Explicit bounds for untrusted clipboard and file input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RtfLimits {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_tokens: usize,
    pub max_text_length: usize,
}

impl Default for RtfLimits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_depth: 128,
            max_tokens: 1_000_000,
            max_text_length: 8 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RtfError {
    InvalidHeader,
    InvalidSyntax {
        offset: usize,
        message: &'static str,
    },
    LimitExceeded(&'static str),
    InvalidUnicode,
    InvalidParameter(&'static str),
    UnsupportedCodePage(i32),
    UnsupportedFeature(&'static str),
}

impl fmt::Display for RtfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader => f.write_str("Expected an RTF version 1 document."),
            Self::InvalidSyntax { offset, message } => {
                write!(f, "Invalid RTF at byte {offset}: {message}")
            }
            Self::LimitExceeded(limit) => write!(f, "RTF {limit} limit exceeded."),
            Self::InvalidUnicode => f.write_str("RTF contains unpaired UTF-16 surrogates."),
            Self::InvalidParameter(parameter) => write!(f, "Invalid RTF {parameter} parameter."),
            Self::UnsupportedCodePage(codepage) => write!(
                f,
                "RTF code page {codepage} is not supported by this codec."
            ),
            Self::UnsupportedFeature(feature) => {
                write!(f, "RTF {feature} cannot be represented by this codec.")
            }
        }
    }
}
impl std::error::Error for RtfError {}

enum Node<'a> {
    Group(Vec<Node<'a>>),
    Control(&'a str, Option<i32>),
    Text(&'a [u8]),
    Byte(u8),
    Symbol(u8),
    Binary,
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
    tokens: usize,
    limits: RtfLimits,
}

impl<'a> Parser<'a> {
    fn syntax(&self, message: &'static str) -> RtfError {
        RtfError::InvalidSyntax {
            offset: self.position,
            message,
        }
    }

    fn group(&mut self, depth: usize) -> Result<Vec<Node<'a>>, RtfError> {
        if depth > self.limits.max_depth.min(512) {
            return Err(RtfError::LimitExceeded("nesting"));
        }
        if self.bytes.get(self.position) != Some(&b'{') {
            return Err(self.syntax("expected opening brace"));
        }
        self.position += 1;
        let mut nodes = Vec::new();
        loop {
            self.tokens += 1;
            if self.tokens > self.limits.max_tokens {
                return Err(RtfError::LimitExceeded("token count"));
            }
            match self.bytes.get(self.position).copied() {
                None => return Err(self.syntax("unclosed group")),
                Some(b'}') => {
                    self.position += 1;
                    return Ok(nodes);
                }
                Some(b'{') => nodes.push(Node::Group(self.group(depth + 1)?)),
                Some(b'\\') => {
                    self.position += 1;
                    let byte = self
                        .bytes
                        .get(self.position)
                        .copied()
                        .ok_or_else(|| self.syntax("incomplete escape"))?;
                    self.position += 1;
                    if byte == b'\'' {
                        let raw = self
                            .bytes
                            .get(self.position..self.position + 2)
                            .ok_or_else(|| self.syntax("incomplete hex escape"))?;
                        let high = hex(raw[0]).ok_or_else(|| self.syntax("invalid hex escape"))?;
                        let low = hex(raw[1]).ok_or_else(|| self.syntax("invalid hex escape"))?;
                        self.position += 2;
                        nodes.push(Node::Byte((high << 4) | low));
                    } else if byte.is_ascii_alphabetic() {
                        let start = self.position - 1;
                        while self
                            .bytes
                            .get(self.position)
                            .is_some_and(u8::is_ascii_alphabetic)
                        {
                            self.position += 1;
                        }
                        let word = std::str::from_utf8(&self.bytes[start..self.position])
                            .map_err(|_| self.syntax("invalid control word"))?;
                        if word.len() > 32 {
                            return Err(self.syntax("control word too long"));
                        }
                        let negative = self.bytes.get(self.position) == Some(&b'-');
                        if negative {
                            self.position += 1;
                        }
                        let number_start = self.position;
                        let mut value = 0_i64;
                        while let Some(byte) = self
                            .bytes
                            .get(self.position)
                            .copied()
                            .filter(u8::is_ascii_digit)
                        {
                            value = value
                                .checked_mul(10)
                                .and_then(|v| v.checked_add((byte - b'0') as i64))
                                .ok_or_else(|| self.syntax("numeric parameter overflow"))?;
                            if value > i32::MAX as i64 + i64::from(negative) {
                                return Err(self.syntax("numeric parameter overflow"));
                            }
                            self.position += 1;
                        }
                        if negative && self.position == number_start {
                            return Err(self.syntax("missing signed numeric parameter"));
                        }
                        let parameter = (self.position != number_start).then_some(if negative {
                            (-value) as i32
                        } else {
                            value as i32
                        });
                        if self.bytes.get(self.position) == Some(&b' ') {
                            self.position += 1;
                        }
                        if word == "bin" {
                            let length = parameter
                                .filter(|value| *value >= 0)
                                .ok_or(RtfError::InvalidParameter("binary length"))?
                                as usize;
                            self.position = self
                                .position
                                .checked_add(length)
                                .filter(|end| *end <= self.bytes.len())
                                .ok_or_else(|| self.syntax("incomplete binary payload"))?;
                            nodes.push(Node::Binary);
                        } else {
                            nodes.push(Node::Control(word, parameter));
                        }
                    } else {
                        nodes.push(Node::Symbol(byte));
                    }
                }
                Some(b'\r' | b'\n') => {
                    self.position += 1;
                }
                Some(_) => {
                    let start = self.position;
                    while self
                        .bytes
                        .get(self.position)
                        .is_some_and(|byte| !matches!(byte, b'{' | b'}' | b'\\' | b'\r' | b'\n'))
                    {
                        self.position += 1;
                    }
                    nodes.push(Node::Text(&self.bytes[start..self.position]));
                }
            }
        }
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Clone)]
struct State {
    character: CharacterFormat,
    paragraph: ParagraphFormat,
    uc: usize,
    skip: usize,
    underline: bool,
    strike: bool,
    line_spacing_twips: i32,
    line_spacing_multiple: bool,
    tab_alignment: TabAlignment,
    tab_leader: TabLeader,
}
impl Default for State {
    fn default() -> Self {
        Self {
            character: CharacterFormat::default(),
            paragraph: ParagraphFormat {
                default_tab_stop: Some(48.0),
                ..Default::default()
            },
            uc: 1,
            skip: 0,
            underline: false,
            strike: false,
            line_spacing_twips: 0,
            line_spacing_multiple: false,
            tab_alignment: TabAlignment::Left,
            tab_leader: TabLeader::Spaces,
        }
    }
}

struct Decoder {
    limits: RtfLimits,
    fonts: BTreeMap<i32, FontFamily>,
    colors: Vec<Option<Rc<dyn IBrush>>>,
    default_font: Option<i32>,
    default_language: Option<CultureInfo>,
    default_tab_stop: Option<f64>,
    document: DocumentSnapshot,
    pending: Vec<u16>,
    pending_format: CharacterFormat,
    text_length: usize,
}

/// A bounded RTF 1 reader for the formatting represented by FerroUI documents.
///
/// Supports scoped character/paragraph properties, font/color tables, signed Unicode
/// escapes, fallback tokens, Unicode alternative destinations, Windows-1252 byte escapes,
/// character spacing in quarter-points or twips, and hyperlink fields. Ignorable destinations
/// are skipped without interpreting their content. Native OLE, pictures, tables, unsupported
/// font encodings, and horizontal glyph scaling return explicit errors.
///
/// Unicode group/fallback rules and character metrics follow Microsoft's RTF specification,
/// sections "Unicode RTF" and "Font (character) Formatting Properties". This codec implements
/// that exchange grammar independently of the external WinUIEdit engine.
pub struct RtfReader;

impl RtfReader {
    pub fn read(bytes: &[u8]) -> Result<DocumentSnapshot, RtfError> {
        Self::read_with_limits(bytes, RtfLimits::default())
    }
    pub fn read_str(text: &str) -> Result<DocumentSnapshot, RtfError> {
        Self::read(text.as_bytes())
    }
    pub fn read_with_limits(bytes: &[u8], limits: RtfLimits) -> Result<DocumentSnapshot, RtfError> {
        if bytes.len() > limits.max_bytes {
            return Err(RtfError::LimitExceeded("input size"));
        }
        let mut parser = Parser {
            bytes,
            position: 0,
            tokens: 0,
            limits,
        };
        let nodes = parser.group(0)?;
        if !bytes[parser.position..].iter().all(u8::is_ascii_whitespace) {
            return Err(parser.syntax("data after root group"));
        }
        if !matches!(nodes.first(), Some(Node::Control("rtf", Some(1)))) {
            return Err(RtfError::InvalidHeader);
        }
        let mut decoder = Decoder {
            limits,
            fonts: BTreeMap::new(),
            colors: Vec::new(),
            default_font: None,
            default_language: None,
            default_tab_stop: Some(48.0),
            document: DocumentSnapshot {
                paragraphs: vec![ParagraphSnapshot {
                    format: ParagraphFormat {
                        default_tab_stop: Some(48.0),
                        ..Default::default()
                    },
                    inlines: Vec::new(),
                }],
            },
            pending: Vec::new(),
            pending_format: CharacterFormat::default(),
            text_length: 0,
        };
        decoder.decode(&nodes, &mut State::default())?;
        decoder.flush()?;
        decoder
            .document
            .validate()
            .map_err(|_| RtfError::InvalidParameter("document formatting"))?;
        Ok(decoder.document)
    }
}

impl Decoder {
    fn paragraph(&mut self) -> &mut ParagraphSnapshot {
        self.document
            .paragraphs
            .last_mut()
            .expect("a document always has a paragraph")
    }
    fn flush(&mut self) -> Result<(), RtfError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let text = String::from_utf16(&self.pending).map_err(|_| RtfError::InvalidUnicode)?;
        self.pending.clear();
        let format = self.pending_format.clone();
        self.paragraph().inlines.push(InlineSnapshot {
            format,
            kind: InlineSnapshotKind::Run(text),
        });
        Ok(())
    }
    fn append(&mut self, unit: u16, state: &State) -> Result<(), RtfError> {
        if self.pending_format != state.character {
            self.flush()?;
            self.pending_format = state.character.clone();
        }
        self.text_length += 1;
        if self.text_length > self.limits.max_text_length {
            return Err(RtfError::LimitExceeded("text length"));
        }
        self.pending.push(unit);
        Ok(())
    }
    fn byte(&mut self, byte: u8, state: &mut State) -> Result<(), RtfError> {
        if state.skip > 0 {
            state.skip -= 1;
            return Ok(());
        }
        self.append(cp1252(byte), state)
    }
    fn format_decorations(state: &mut State) {
        state.character.text_decorations = Some(if state.underline && state.strike {
            Some(TextDecorationCollection::from_items([
                TextDecorations::underline().get(0),
                TextDecorations::strikethrough().get(0),
            ]))
        } else if state.underline {
            Some(TextDecorations::underline())
        } else if state.strike {
            Some(TextDecorations::strikethrough())
        } else {
            None
        });
    }
    fn decode(&mut self, nodes: &[Node<'_>], state: &mut State) -> Result<(), RtfError> {
        for node in nodes {
            // RTF Unicode fallback counts every control word, symbol or binary
            // token as one character. A scope delimiter ends the fallback run.
            if matches!(node, Node::Group(_)) {
                state.skip = 0;
            } else if state.skip > 0 && !matches!(node, Node::Text(_)) {
                state.skip -= 1;
                continue;
            }
            match node {
                Node::Text(bytes) => {
                    for byte in *bytes {
                        self.byte(*byte, state)?;
                    }
                }
                Node::Byte(byte) => self.byte(*byte, state)?,
                Node::Symbol(symbol) => match symbol {
                    b'\\' | b'{' | b'}' => self.byte(*symbol, state)?,
                    b'~' => self.character(0x00a0, state)?,
                    b'_' => self.character(0x2011, state)?,
                    b'-' => self.character(0x00ad, state)?,
                    b'\r' | b'\n' => {}
                    _ => {}
                },
                Node::Binary => return Err(RtfError::UnsupportedFeature("binary content")),
                Node::Group(children) => {
                    let destination = destination(children);
                    match destination {
                        Some(("fonttbl", _)) => self.read_fonts(children, state.uc)?,
                        Some(("colortbl", _)) => self.read_colors(children)?,
                        Some(("field", _)) => self.read_field(children, state)?,
                        Some(("upr", _)) => {
                            let unicode = unicode_alternative(children)?;
                            self.decode(unicode, &mut state.clone())?;
                        }
                        Some(("pict" | "object" | "shppict" | "nonshppict", _)) => {
                            return Err(RtfError::UnsupportedFeature("images or embedded objects"));
                        }
                        Some((
                            "stylesheet" | "info" | "generator" | "themedata"
                            | "colorschememapping" | "xmlnstbl" | "datastore" | "latentstyles"
                            | "header" | "footer" | "headerl" | "headerr" | "footerl" | "footerr",
                            _,
                        )) => {}
                        Some(("listtable" | "listoverridetable", _)) => {
                            return Err(RtfError::UnsupportedFeature("list definitions"));
                        }
                        Some((_, true)) => {}
                        _ => {
                            let mut child_state = state.clone();
                            child_state.skip = 0;
                            self.decode(children, &mut child_state)?;
                            state.skip = 0;
                        }
                    }
                }
                Node::Control(word, number) => {
                    let number = *number;
                    let enabled = number != Some(0);
                    match *word {
                        "rtf" | "ansi" | "viewkind" | "viewscale" | "deflangfe" | "nouicompat"
                        | "fet" => {}
                        "deflang" => {
                            let language = rtf_language(number)?;
                            self.default_language = Some(language.clone());
                            state.character.language = Some(language);
                        }
                        "lang" => state.character.language = Some(rtf_language(number)?),
                        "ansicpg" => {
                            let codepage = number.ok_or(RtfError::InvalidParameter("code page"))?;
                            if codepage != 1252 {
                                return Err(RtfError::UnsupportedCodePage(codepage));
                            }
                        }
                        "mac" => return Err(RtfError::UnsupportedCodePage(10000)),
                        "pc" => return Err(RtfError::UnsupportedCodePage(437)),
                        "pca" => return Err(RtfError::UnsupportedCodePage(850)),
                        "uc" => {
                            state.uc = number
                                .filter(|n| (0..=255).contains(n))
                                .ok_or(RtfError::InvalidParameter("Unicode fallback length"))?
                                as usize
                        }
                        "u" => {
                            let value = number
                                .filter(|n| (-32768..=65535).contains(n))
                                .ok_or(RtfError::InvalidParameter("Unicode code unit"))?;
                            self.append(value as u16, state)?;
                            state.skip = state.uc;
                        }
                        "deff" => self.default_font = number,
                        "plain" => {
                            state.character = CharacterFormat::default();
                            state.character.language = self.default_language.clone();
                            state.character.font_family = self
                                .default_font
                                .and_then(|id| self.fonts.get(&id).cloned());
                            state.character.font_weight = Some(FontWeight::Normal);
                            state.character.font_style = Some(FontStyle::Normal);
                            state.character.font_size = Some(16.0);
                            state.character.font_stretch = Some(FontStretch::Normal);
                            state.character.letter_spacing = Some(0.0);
                            state.character.foreground = Some(None);
                            state.character.background = Some(None);
                            state.character.baseline_alignment = Some(BaselineAlignment::Baseline);
                            state.underline = false;
                            state.strike = false;
                            Self::format_decorations(state);
                        }
                        "b" => {
                            state.character.font_weight = Some(if enabled {
                                FontWeight::Bold
                            } else {
                                FontWeight::Normal
                            })
                        }
                        "i" => {
                            state.character.font_style = Some(if enabled {
                                FontStyle::Italic
                            } else {
                                FontStyle::Normal
                            })
                        }
                        "ul" => {
                            state.underline = enabled;
                            Self::format_decorations(state);
                        }
                        "ulnone" => {
                            state.underline = false;
                            Self::format_decorations(state);
                        }
                        "strike" => {
                            state.strike = enabled;
                            Self::format_decorations(state);
                        }
                        "f" => {
                            let id = number.ok_or(RtfError::InvalidParameter("font"))?;
                            state.character.font_family = Some(
                                self.fonts
                                    .get(&id)
                                    .cloned()
                                    .ok_or(RtfError::InvalidParameter("font table reference"))?,
                            );
                        }
                        "fs" => {
                            state.character.font_size = Some(
                                number
                                    .filter(|n| *n > 0)
                                    .ok_or(RtfError::InvalidParameter("font size"))?
                                    as f64
                                    * (2.0 / 3.0),
                            )
                        }
                        "cf" | "highlight" | "cb" => {
                            let index = number
                                .filter(|n| *n >= 0)
                                .ok_or(RtfError::InvalidParameter("color"))?
                                as usize;
                            let color = if index == 0 && self.colors.is_empty() {
                                None
                            } else {
                                self.colors
                                    .get(index)
                                    .cloned()
                                    .ok_or(RtfError::InvalidParameter("color table reference"))?
                            };
                            if *word == "cf" {
                                state.character.foreground = Some(color);
                            } else {
                                state.character.background = Some(color);
                            }
                        }
                        "super" => {
                            state.character.baseline_alignment =
                                Some(BaselineAlignment::Superscript)
                        }
                        "sub" => {
                            state.character.baseline_alignment = Some(BaselineAlignment::Subscript)
                        }
                        "nosupersub" => {
                            state.character.baseline_alignment = Some(BaselineAlignment::Baseline)
                        }
                        "charscalex" => {
                            let scale = number
                                .filter(|value| *value > 0)
                                .ok_or(RtfError::InvalidParameter("character scaling"))?;
                            if scale != 100 {
                                // Horizontal glyph scaling is not a font-face
                                // stretch selection and cannot be substituted.
                                return Err(RtfError::UnsupportedFeature("character scaling"));
                            }
                            state.character.font_stretch = Some(FontStretch::Normal);
                        }
                        "expnd" | "expndtw" => {
                            state.character.letter_spacing = Some(
                                number.ok_or(RtfError::InvalidParameter("character spacing"))?
                                    as f64
                                    / if *word == "expnd" { 3.0 } else { 15.0 },
                            )
                        }
                        "pard" => {
                            state.paragraph = ParagraphFormat {
                                default_tab_stop: self.default_tab_stop,
                                ..Default::default()
                            };
                            state.tab_alignment = TabAlignment::Left;
                            state.tab_leader = TabLeader::Spaces;
                            state.line_spacing_twips = 0;
                            state.line_spacing_multiple = false;
                            self.paragraph().format = state.paragraph.clone();
                        }
                        "ql" | "qr" | "qc" | "qj" => {
                            state.paragraph.text_alignment = Some(match *word {
                                "qr" => TextAlignment::Right,
                                "qc" => TextAlignment::Center,
                                "qj" => TextAlignment::Justify,
                                _ => TextAlignment::Left,
                            });
                            self.paragraph().format = state.paragraph.clone();
                        }
                        "ltrpar" | "rtlpar" => {
                            state.paragraph.flow_direction = Some(if *word == "rtlpar" {
                                FlowDirection::RightToLeft
                            } else {
                                FlowDirection::LeftToRight
                            });
                            self.paragraph().format = state.paragraph.clone();
                        }
                        "fi" => {
                            state.paragraph.text_indent = Some(
                                number.ok_or(RtfError::InvalidParameter("paragraph indent"))?
                                    as f64
                                    / 15.0,
                            );
                            self.paragraph().format = state.paragraph.clone();
                        }
                        "li" | "ri" => {
                            let value = f64::from(
                                number.ok_or(RtfError::InvalidParameter("paragraph indent"))?,
                            ) / 15.0;
                            if *word == "li" {
                                state.paragraph.left_indent = Some(value);
                            } else {
                                state.paragraph.right_indent = Some(value);
                            }
                            self.paragraph().format = state.paragraph.clone();
                        }
                        "sb" | "sa" => {
                            let value = number
                                .ok_or(RtfError::InvalidParameter("paragraph margin"))?
                                as f64
                                / 15.0;
                            let mut margin = state.paragraph.margin.unwrap_or_default();
                            match *word {
                                "sb" => margin.top = value,
                                _ => margin.bottom = value,
                            }
                            state.paragraph.margin = Some(margin);
                            self.paragraph().format = state.paragraph.clone();
                        }
                        "deftab" => {
                            let value = number
                                .filter(|value| *value > 0)
                                .ok_or(RtfError::InvalidParameter("default tab spacing"))?
                                as f64
                                / 15.0;
                            self.default_tab_stop = Some(value);
                            state.paragraph.default_tab_stop = Some(value);
                            for paragraph in &mut self.document.paragraphs {
                                paragraph.format.default_tab_stop = Some(value);
                            }
                        }
                        "tql" | "tqc" | "tqr" | "tqdec" => {
                            state.tab_alignment = match *word {
                                "tqc" => TabAlignment::Center,
                                "tqr" => TabAlignment::Right,
                                "tqdec" => TabAlignment::Decimal,
                                _ => TabAlignment::Left,
                            };
                        }
                        "tldot" | "tlhyph" | "tlul" | "tlth" | "tleq" => {
                            state.tab_leader = match *word {
                                "tldot" => TabLeader::Dots,
                                "tlhyph" => TabLeader::Dashes,
                                "tlul" => TabLeader::Lines,
                                "tlth" => TabLeader::ThickLines,
                                _ => TabLeader::Equals,
                            };
                        }
                        "tlmdot" => {
                            return Err(RtfError::UnsupportedFeature("middle-dot tab leaders"))
                        }
                        "tx" | "tb" => {
                            let position = number
                                .filter(|value| *value >= 0)
                                .ok_or(RtfError::InvalidParameter("tab position"))?
                                as f64
                                / 15.0;
                            let stop = TextTabStop::new(
                                position,
                                if *word == "tb" {
                                    TabAlignment::Bar
                                } else {
                                    state.tab_alignment
                                },
                                state.tab_leader,
                            );
                            let tabs = state
                                .paragraph
                                .tab_stops
                                .clone()
                                .unwrap_or_default()
                                .with_tab(stop)
                                .map_err(|_| RtfError::InvalidParameter("tab stops"))?;
                            state.paragraph.tab_stops = Some(tabs);
                            state.tab_alignment = TabAlignment::Left;
                            state.tab_leader = TabLeader::Spaces;
                            self.paragraph().format = state.paragraph.clone();
                        }
                        "sl" => {
                            let value = number.ok_or(RtfError::InvalidParameter("line height"))?;
                            state.line_spacing_twips = value;
                            self.set_line_spacing(state)?;
                        }
                        "slmult" => {
                            state.line_spacing_multiple = match number {
                                Some(0) => false,
                                Some(1) => true,
                                _ => {
                                    return Err(RtfError::InvalidParameter("multiple line spacing"))
                                }
                            };
                            self.set_line_spacing(state)?;
                        }
                        "par" => {
                            self.flush()?;
                            self.text_length += 1;
                            if self.text_length > self.limits.max_text_length {
                                return Err(RtfError::LimitExceeded("text length"));
                            }
                            self.document.paragraphs.push(ParagraphSnapshot {
                                format: state.paragraph.clone(),
                                inlines: Vec::new(),
                            });
                        }
                        "line" => {
                            self.flush()?;
                            self.text_length += 1;
                            if self.text_length > self.limits.max_text_length {
                                return Err(RtfError::LimitExceeded("text length"));
                            }
                            let format = state.character.clone();
                            self.paragraph().inlines.push(InlineSnapshot {
                                format,
                                kind: InlineSnapshotKind::LineBreak,
                            });
                        }
                        "tab" => self.character(b'\t' as u16, state)?,
                        "emdash" => self.character(0x2014, state)?,
                        "endash" => self.character(0x2013, state)?,
                        "emspace" => self.character(0x2003, state)?,
                        "enspace" => self.character(0x2002, state)?,
                        "bullet" => self.character(0x2022, state)?,
                        "lquote" => self.character(0x2018, state)?,
                        "rquote" => self.character(0x2019, state)?,
                        "ldblquote" => self.character(0x201c, state)?,
                        "rdblquote" => self.character(0x201d, state)?,
                        "trowd" | "cell" | "row" | "intbl" => {
                            return Err(RtfError::UnsupportedFeature("tables"));
                        }
                        "pict" | "object" => {
                            return Err(RtfError::UnsupportedFeature("images or embedded objects"));
                        }
                        // RTF readers are required to ignore unrecognized control words. Explicit
                        // errors above cover content-bearing structures this model cannot represent.
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
    fn character(&mut self, unit: u16, state: &mut State) -> Result<(), RtfError> {
        if state.skip > 0 {
            state.skip -= 1;
            return Ok(());
        }
        self.append(unit, state)
    }
    fn set_line_spacing(&mut self, state: &mut State) -> Result<(), RtfError> {
        let value = state.line_spacing_twips;
        if state.line_spacing_multiple && value < 0 {
            return Err(RtfError::InvalidParameter("multiple line spacing"));
        }
        let (rule, spacing) = if state.line_spacing_multiple && value != 0 {
            (LineSpacingRule::Multiple, f64::from(value) / 240.0)
        } else if value == 0 {
            (LineSpacingRule::Single, 0.0)
        } else if value < 0 {
            (LineSpacingRule::Exactly, f64::from(value).abs() / 15.0)
        } else {
            (LineSpacingRule::AtLeast, f64::from(value) / 15.0)
        };
        state.paragraph.line_spacing_rule = Some(rule);
        state.paragraph.line_spacing = Some(spacing);
        // Retain the established block metrics for callers of the original
        // paragraph API; the explicit rule controls the rich layout path.
        state.paragraph.line_height =
            (!state.line_spacing_multiple && value != 0).then_some(f64::from(value).abs() / 15.0);
        state.paragraph.line_stacking_strategy = Some(if value < 0 {
            LineStackingStrategy::BlockLineHeight
        } else {
            LineStackingStrategy::MaxHeight
        });
        self.paragraph().format = state.paragraph.clone();
        Ok(())
    }
    fn read_fonts(&mut self, nodes: &[Node<'_>], uc: usize) -> Result<(), RtfError> {
        fn visit(
            nodes: &[Node<'_>],
            fonts: &mut BTreeMap<i32, FontFamily>,
            uc: usize,
            limit: usize,
        ) -> Result<(), RtfError> {
            let mut id = None;
            let mut name = DestinationText::new(uc, limit);
            for node in nodes {
                if matches!(node, Node::Group(_)) {
                    name.skip = 0;
                } else if name.skip > 0 && !matches!(node, Node::Text(_)) {
                    name.node(node)?;
                    continue;
                }
                match node {
                    Node::Control("f", Some(number)) => {
                        if *number < 0 {
                            return Err(RtfError::InvalidParameter("font identifier"));
                        }
                        id = Some(*number);
                        name.text.clear();
                    }
                    Node::Control("fcharset", Some(value)) if !matches!(value, 0 | 1) => {
                        return Err(RtfError::UnsupportedFeature("font character set"));
                    }
                    Node::Control("cpg", Some(value)) if *value != 1252 => {
                        return Err(RtfError::UnsupportedCodePage(*value));
                    }
                    Node::Group(children)
                        if destination(children).is_some_and(|(word, _)| word == "upr") =>
                    {
                        name.node(node)?;
                    }
                    Node::Group(children) => {
                        if !destination(children).is_some_and(|(_, starred)| starred) {
                            visit(children, fonts, name.uc, limit)?;
                        }
                    }
                    Node::Text(bytes) => {
                        for byte in *bytes {
                            if *byte == b';' && name.skip == 0 {
                                if let Some(id) = id {
                                    let value = name.string()?;
                                    if !value.trim().is_empty() {
                                        fonts.insert(id, FontFamily::new(value.trim()));
                                    }
                                }
                                name.text.clear();
                            } else {
                                name.append(cp1252(*byte))?;
                            }
                        }
                    }
                    _ => name.node(node)?,
                }
            }
            Ok(())
        }
        visit(nodes, &mut self.fonts, uc, self.limits.max_text_length)
    }
    fn read_colors(&mut self, nodes: &[Node<'_>]) -> Result<(), RtfError> {
        let (mut red, mut green, mut blue, mut has_color) = (0, 0, 0, false);
        for node in nodes {
            match node {
                Node::Control(word @ ("red" | "green" | "blue"), Some(value)) => {
                    let value = u8::try_from(*value)
                        .map_err(|_| RtfError::InvalidParameter("color component"))?;
                    match *word {
                        "red" => red = value,
                        "green" => green = value,
                        _ => blue = value,
                    };
                    has_color = true;
                }
                Node::Text(bytes) => {
                    for byte in *bytes {
                        if *byte == b';' {
                            self.colors.push(if has_color {
                                Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                                    red, green, blue,
                                ))) as Rc<dyn IBrush>)
                            } else {
                                None
                            });
                            red = 0;
                            green = 0;
                            blue = 0;
                            has_color = false;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn read_field(&mut self, nodes: &[Node<'_>], state: &State) -> Result<(), RtfError> {
        self.flush()?;
        let mut instruction = String::new();
        let mut result = None;
        for node in nodes {
            if let Node::Group(children) = node {
                match destination(children) {
                    Some(("fldinst", _)) => {
                        let mut text = DestinationText::new(state.uc, self.limits.max_text_length);
                        text.nodes(children)?;
                        instruction = text.string()?;
                    }
                    Some(("fldrslt", _)) => result = Some(children),
                    _ => {}
                }
            }
        }
        let target = parse_hyperlink(&instruction)?;
        if let Some(nodes) = result {
            let paragraph_index = self.document.paragraphs.len() - 1;
            let inline_index = self.paragraph().inlines.len();
            self.decode(nodes, &mut state.clone())?;
            self.flush()?;
            if let Some(target) = target {
                if self.document.paragraphs.len() - 1 != paragraph_index {
                    return Err(RtfError::UnsupportedFeature("hyperlinks across paragraphs"));
                }
                let inlines = self.paragraph().inlines.split_off(inline_index);
                self.paragraph().inlines.push(InlineSnapshot {
                    format: CharacterFormat::default(),
                    kind: InlineSnapshotKind::Hyperlink {
                        inlines,
                        navigate_uri: Some(target.uri),
                        target_name: target.name,
                        is_enabled: true,
                        is_tab_stop: true,
                        tab_index: 0,
                        is_visited: false,
                        underline_style: crate::documents::UnderlineStyle::Single,
                    },
                });
            }
        }
        Ok(())
    }
}

fn destination<'a>(nodes: &'a [Node<'a>]) -> Option<(&'a str, bool)> {
    let starred = matches!(nodes.first(), Some(Node::Symbol(b'*')));
    match nodes.get(usize::from(starred)) {
        Some(Node::Control(word, _)) => Some((word, starred)),
        _ => None,
    }
}

fn unicode_alternative<'a>(nodes: &'a [Node<'a>]) -> Result<&'a [Node<'a>], RtfError> {
    nodes
        .iter()
        .find_map(|node| match node {
            Node::Group(children)
                if destination(children).is_some_and(|(word, _)| word == "ud") =>
            {
                Some(children.as_slice())
            }
            _ => None,
        })
        .ok_or(RtfError::InvalidParameter(
            "Unicode alternative destination",
        ))
}
/// Decodes text destinations without interpreting their font/field control words as
/// document formatting. The Unicode fallback and group scope rules are identical to body text.
struct DestinationText {
    text: Vec<u16>,
    uc: usize,
    skip: usize,
    limit: usize,
}
impl DestinationText {
    fn new(uc: usize, limit: usize) -> Self {
        Self {
            text: Vec::new(),
            uc,
            skip: 0,
            limit,
        }
    }
    fn append(&mut self, value: u16) -> Result<(), RtfError> {
        if self.skip > 0 {
            self.skip -= 1;
            return Ok(());
        }
        if self.text.len() >= self.limit {
            return Err(RtfError::LimitExceeded("destination text length"));
        }
        self.text.push(value);
        Ok(())
    }
    fn string(&self) -> Result<String, RtfError> {
        String::from_utf16(&self.text).map_err(|_| RtfError::InvalidUnicode)
    }
    fn nodes(&mut self, nodes: &[Node<'_>]) -> Result<(), RtfError> {
        for node in nodes {
            self.node(node)?;
        }
        Ok(())
    }
    fn node(&mut self, node: &Node<'_>) -> Result<(), RtfError> {
        if matches!(node, Node::Group(_)) {
            self.skip = 0;
        } else if self.skip > 0 && !matches!(node, Node::Text(_)) {
            self.skip -= 1;
            return Ok(());
        }
        match node {
            Node::Text(bytes) => {
                for byte in *bytes {
                    self.append(cp1252(*byte))?;
                }
            }
            Node::Byte(byte) => self.append(cp1252(*byte))?,
            Node::Symbol(byte @ (b'\\' | b'{' | b'}')) => self.append(*byte as u16)?,
            Node::Symbol(b'~') => self.append(0xa0)?,
            Node::Symbol(b'_') => self.append(0x2011)?,
            Node::Symbol(b'-') => self.append(0xad)?,
            Node::Control("tab", _) => self.append(9)?,
            Node::Control("u", value) => {
                let value = value
                    .filter(|value| (-32768..=65535).contains(value))
                    .ok_or(RtfError::InvalidParameter("Unicode code unit"))?;
                self.skip = 0;
                self.append(value as u16)?;
                self.skip = self.uc;
            }
            Node::Control("uc", value) => {
                self.uc = value
                    .filter(|value| (0..=255).contains(value))
                    .ok_or(RtfError::InvalidParameter("Unicode fallback length"))?
                    as usize
            }
            Node::Group(children) => {
                let uc = self.uc;
                self.skip = 0;
                if destination(children).is_some_and(|(word, _)| word == "upr") {
                    self.nodes(unicode_alternative(children)?)?;
                } else {
                    self.nodes(children)?;
                }
                self.uc = uc;
                self.skip = 0;
            }
            Node::Binary => return Err(RtfError::UnsupportedFeature("binary text destination")),
            _ => {}
        }
        Ok(())
    }
}
struct HyperlinkTarget {
    uri: String,
    name: Option<String>,
}
fn parse_hyperlink(instruction: &str) -> Result<Option<HyperlinkTarget>, RtfError> {
    let instruction = instruction.trim();
    if instruction.is_empty() {
        return Ok(None);
    }
    let (name, rest) = instruction
        .split_once(char::is_whitespace)
        .unwrap_or((instruction, ""));
    if !name.eq_ignore_ascii_case("HYPERLINK") {
        return Ok(None);
    }
    let mut rest = rest.trim();
    let mut tokens = Vec::new();
    while !rest.is_empty() {
        if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted
                .find('"')
                .ok_or(RtfError::InvalidParameter("hyperlink field"))?;
            tokens.push(quoted[..end].to_owned());
            rest = quoted[end + 1..].trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            tokens.push(rest[..end].to_owned());
            rest = rest[end..].trim_start();
        }
    }
    let mut uri = String::new();
    let (mut name, mut bookmark) = (None, None);
    let mut tokens = tokens.into_iter();
    while let Some(token) = tokens.next() {
        match token.as_str() {
            "\\t" => {
                name = Some(
                    tokens
                        .next()
                        .ok_or(RtfError::InvalidParameter("hyperlink target"))?,
                )
            }
            "\\l" => {
                bookmark = Some(
                    tokens
                        .next()
                        .ok_or(RtfError::InvalidParameter("hyperlink bookmark"))?,
                )
            }
            value if value.starts_with('\\') => {
                return Err(RtfError::UnsupportedFeature("hyperlink field switch"));
            }
            value if uri.is_empty() => uri = value.to_owned(),
            _ => return Err(RtfError::InvalidParameter("hyperlink field")),
        }
    }
    if let Some(bookmark) = bookmark {
        uri.push('#');
        uri.push_str(&bookmark);
    }
    if uri.is_empty() {
        return Err(RtfError::InvalidParameter("hyperlink URI"));
    }
    Ok(Some(HyperlinkTarget { uri, name }))
}
fn cp1252(byte: u8) -> u16 {
    const UPPER: [u16; 32] = [
        0x20ac, 0x81, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160,
        0x2039, 0x0152, 0x8d, 0x017d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013,
        0x2014, 0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x9d, 0x017e, 0x0178,
    ];
    if (0x80..=0x9f).contains(&byte) {
        UPPER[(byte - 0x80) as usize]
    } else {
        byte as u16
    }
}

/// Produces ASCII RTF with signed UTF-16 escapes, font/color tables, scoped formatting,
/// paragraph metrics, and hyperlink fields. Unsupported content returns an error; the
/// in-process clipboard format remains available for lossless structured transfers.
pub struct RtfWriter;

struct Encoder {
    fonts: Vec<FontFamily>,
    colors: Vec<Color>,
    output: String,
}

impl RtfWriter {
    /// Writes a self-contained snapshot. Tabbed snapshots must assign their
    /// paragraph intervals; an unassigned value cannot identify a layout host.
    /// Use write_with_default_tab_stop for a known document default, or export
    /// through RichTextBlock to resolve its font-scaled automatic intervals.
    pub fn write(document: &DocumentSnapshot) -> Result<String, RtfError> {
        Self::write_with_tab_context(document, None)
    }

    /// Supplies the host interval, in DIPs, for unassigned paragraph defaults.
    /// Explicit paragraph intervals still win. RTF represents only one interval
    /// per document, so differing resolved intervals are reported as unsupported.
    pub fn write_with_default_tab_stop(
        document: &DocumentSnapshot,
        default_tab_stop: f64,
    ) -> Result<String, RtfError> {
        if !default_tab_stop.is_finite() || default_tab_stop <= 0.0 {
            return Err(RtfError::InvalidParameter("default tab spacing"));
        }
        Self::write_with_tab_context(document, Some(default_tab_stop))
    }

    fn write_with_tab_context(
        document: &DocumentSnapshot,
        default_tab_stop: Option<f64>,
    ) -> Result<String, RtfError> {
        document
            .validate()
            .map_err(|_| RtfError::InvalidParameter("document formatting"))?;
        let mut encoder = Encoder {
            fonts: Vec::new(),
            colors: Vec::new(),
            output: String::new(),
        };
        for paragraph in &document.paragraphs {
            encoder.collect_format(&paragraph.format.character)?;
            encoder.collect_inlines(&paragraph.inlines, 0)?;
        }
        encoder.output.push_str("{\\rtf1\\ansi\\ansicpg1252\\uc1");
        if default_tab_stop.is_none()
            && document.paragraphs.iter().any(|paragraph| {
                paragraph.format.default_tab_stop.is_none() && paragraph.text().contains('\t')
            })
        {
            return Err(RtfError::UnsupportedFeature(
                "automatic default tab spacing requires a layout context",
            ));
        }
        if let Some(default_tab) = document
            .paragraphs
            .iter()
            .find_map(|p| p.format.default_tab_stop.or(default_tab_stop))
        {
            if document.paragraphs.iter().any(|p| {
                p.format
                    .default_tab_stop
                    .or(default_tab_stop)
                    .unwrap_or(48.0)
                    != default_tab
            }) {
                return Err(RtfError::UnsupportedFeature(
                    "different default tab spacing across paragraphs",
                ));
            }
            let twips = rtf_measure(default_tab, 15.0, "default tab spacing")?;
            if twips <= 0 {
                return Err(RtfError::InvalidParameter("default tab spacing"));
            }
            let _ = write!(encoder.output, "\\deftab{twips}");
        }
        if !encoder.fonts.is_empty() {
            encoder.output.push_str("{\\fonttbl");
            for (index, font) in encoder.fonts.iter().enumerate() {
                let _ = write!(encoder.output, "{{\\f{index}\\fnil ");
                for (part, segment) in font.name().split(';').enumerate() {
                    if part > 0 {
                        encoder.output.push_str("\\u59?");
                    }
                    escape(segment, &mut encoder.output);
                }
                encoder.output.push_str(";}");
            }
            encoder.output.push('}');
        }
        encoder.output.push_str("{\\colortbl;");
        for color in &encoder.colors {
            let _ = write!(
                encoder.output,
                "\\red{}\\green{}\\blue{};",
                color.r, color.g, color.b
            );
        }
        encoder.output.push('}');
        for (index, paragraph) in document.paragraphs.iter().enumerate() {
            if index > 0 {
                encoder.output.push_str("\\par\n");
            }
            encoder.output.push_str("\\pard\\plain");
            encoder.paragraph_format(&paragraph.format)?;
            encoder.format(&paragraph.format.character)?;
            encoder.output.push(' ');
            encoder.inlines(&paragraph.inlines)?;
        }
        encoder.output.push('}');
        Ok(encoder.output)
    }
}

impl Encoder {
    fn collect_format(&mut self, format: &CharacterFormat) -> Result<(), RtfError> {
        if let Some(font) = &format.font_family {
            if !self.fonts.contains(font) {
                self.fonts.push(font.clone());
            }
        }
        for brush in [&format.foreground, &format.background] {
            if let Some(Some(brush)) = brush {
                let color = brush
                    .as_solid_color_brush()
                    .ok_or(RtfError::UnsupportedFeature("non-solid text brushes"))?
                    .color();
                if color.a != 255 || brush.opacity() != 1.0 {
                    return Err(RtfError::UnsupportedFeature("transparent text colors"));
                }
                if !self.colors.contains(&color) {
                    self.colors.push(color);
                }
            }
        }
        if format.font_features.as_ref().is_some_and(Option::is_some) {
            return Err(RtfError::UnsupportedFeature("OpenType feature overrides"));
        }
        if format
            .font_stretch
            .is_some_and(|stretch| stretch != FontStretch::Normal)
        {
            return Err(RtfError::UnsupportedFeature("font stretch"));
        }
        Ok(())
    }
    fn collect_inlines(
        &mut self,
        inlines: &[InlineSnapshot],
        depth: usize,
    ) -> Result<(), RtfError> {
        if depth > 128 {
            return Err(RtfError::LimitExceeded("nesting"));
        }
        for inline in inlines {
            self.collect_format(&inline.format)?;
            match &inline.kind {
                InlineSnapshotKind::Span(children)
                | InlineSnapshotKind::Bold(children)
                | InlineSnapshotKind::Italic(children)
                | InlineSnapshotKind::Underline(children)
                | InlineSnapshotKind::Hyperlink {
                    inlines: children, ..
                } => self.collect_inlines(children, depth + 1)?,
                InlineSnapshotKind::Embedded(_) => {
                    return Err(RtfError::UnsupportedFeature("embedded controls"));
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn format(&mut self, format: &CharacterFormat) -> Result<(), RtfError> {
        if let Some(language) = &format.language {
            let identifier = if language.is_invariant() {
                1024
            } else {
                language.lcid()
            };
            if identifier == 0x1000 {
                return Err(RtfError::UnsupportedFeature(
                    "language tag without a known Windows identifier",
                ));
            }
            let _ = write!(self.output, "\\lang{identifier}");
        }
        if let Some(font) = &format.font_family {
            let index = self
                .fonts
                .iter()
                .position(|candidate| candidate == font)
                .ok_or(RtfError::InvalidParameter("font table"))?;
            let _ = write!(self.output, "\\f{index}");
        }
        if let Some(size) = format.font_size {
            if !size.is_finite() || size <= 0.0 {
                return Err(RtfError::InvalidParameter("font size"));
            }
            let size = rtf_measure(size, 1.5, "font size")?;
            if size <= 0 {
                return Err(RtfError::InvalidParameter("font size"));
            }
            let _ = write!(self.output, "\\fs{size}");
        }
        if format.font_stretch == Some(FontStretch::Normal) {
            self.output.push_str("\\charscalex100");
        }
        if let Some(weight) = format.font_weight {
            if !matches!(weight, FontWeight::Normal | FontWeight::Bold) {
                return Err(RtfError::UnsupportedFeature("intermediate font weights"));
            }
            let _ = write!(self.output, "\\b{}", i32::from(weight == FontWeight::Bold));
        }
        if let Some(style) = format.font_style {
            if style == FontStyle::Oblique {
                return Err(RtfError::UnsupportedFeature("oblique font style"));
            }
            let _ = write!(self.output, "\\i{}", i32::from(style == FontStyle::Italic));
        }
        for (word, brush) in [
            ("cf", &format.foreground),
            ("highlight", &format.background),
        ] {
            if let Some(brush) = brush {
                let index = if let Some(brush) = brush {
                    let color = brush
                        .as_solid_color_brush()
                        .ok_or(RtfError::UnsupportedFeature("non-solid text brushes"))?
                        .color();
                    self.colors
                        .iter()
                        .position(|candidate| candidate == &color)
                        .ok_or(RtfError::InvalidParameter("color table"))?
                        + 1
                } else {
                    0
                };
                let _ = write!(self.output, "\\{word}{index}");
            }
        }
        if let Some(decorations) = &format.text_decorations {
            let (mut underline, mut strike) = (false, false);
            if let Some(decorations) = decorations {
                for index in 0..decorations.len() {
                    match decorations.get(index).location() {
                        TextDecorationLocation::Underline => underline = true,
                        TextDecorationLocation::Strikethrough => strike = true,
                        _ => {
                            return Err(RtfError::UnsupportedFeature(
                                "overline or baseline decorations",
                            ));
                        }
                    }
                }
            }
            let _ = write!(
                self.output,
                "\\ul{}\\strike{}",
                i32::from(underline),
                i32::from(strike)
            );
        }
        if let Some(baseline) = format.baseline_alignment {
            self.output.push_str(match baseline {
                BaselineAlignment::Baseline => "\\nosupersub",
                BaselineAlignment::Superscript => "\\super",
                BaselineAlignment::Subscript => "\\sub",
                _ => return Err(RtfError::UnsupportedFeature("text baseline alignment")),
            });
        }
        if let Some(spacing) = format.letter_spacing {
            let quarters = rtf_measure(spacing, 3.0, "character spacing")?;
            let twips = rtf_measure(spacing, 15.0, "character spacing")?;
            let _ = write!(self.output, "\\expnd{quarters}\\expndtw{twips}");
        }
        Ok(())
    }
    fn paragraph_format(&mut self, format: &ParagraphFormat) -> Result<(), RtfError> {
        if let Some(alignment) = format.text_alignment {
            self.output.push_str(match alignment {
                TextAlignment::Left | TextAlignment::Start => "\\ql",
                TextAlignment::Center => "\\qc",
                TextAlignment::Right | TextAlignment::End => "\\qr",
                TextAlignment::Justify => "\\qj",
                _ => "\\ql",
            });
        }
        if let Some(direction) = format.flow_direction {
            self.output
                .push_str(if direction == FlowDirection::RightToLeft {
                    "\\rtlpar"
                } else {
                    "\\ltrpar"
                });
        }
        let margin = format.margin.unwrap_or(Thickness::default());
        for (word, value) in [
            ("li", margin.left + format.left_indent.unwrap_or(0.0)),
            ("ri", margin.right + format.right_indent.unwrap_or(0.0)),
            ("sb", margin.top),
            ("sa", margin.bottom),
            ("fi", format.text_indent.unwrap_or(0.0)),
        ] {
            if !value.is_finite() {
                return Err(RtfError::InvalidParameter("paragraph metric"));
            }
            if value != 0.0 {
                let value = rtf_measure(value, 15.0, "paragraph metric")?;
                let _ = write!(self.output, "\\{word}{value}");
            }
        }
        if let Some(tabs) = &format.tab_stops {
            for tab in tabs.as_slice() {
                self.output.push_str(match tab.alignment {
                    TabAlignment::Left | TabAlignment::Bar => "",
                    TabAlignment::Center => "\\tqc",
                    TabAlignment::Right => "\\tqr",
                    TabAlignment::Decimal => "\\tqdec",
                });
                self.output.push_str(match tab.leader {
                    TabLeader::Spaces => "",
                    TabLeader::Dots => "\\tldot",
                    TabLeader::Dashes => "\\tlhyph",
                    TabLeader::Lines => "\\tlul",
                    TabLeader::ThickLines => "\\tlth",
                    TabLeader::Equals => "\\tleq",
                });
                let position = rtf_measure(tab.position, 15.0, "tab position")?;
                let word = if tab.alignment == TabAlignment::Bar {
                    "tb"
                } else {
                    "tx"
                };
                let _ = write!(self.output, "\\{word}{position}");
            }
        }
        let rule = format.line_spacing_rule.unwrap_or_default();
        if rule != LineSpacingRule::Undefined {
            let spacing = format.line_spacing.unwrap_or(1.0);
            let (value, multiple) = match rule {
                LineSpacingRule::Single => (240, true),
                LineSpacingRule::OneAndHalf => (360, true),
                LineSpacingRule::Double => (480, true),
                LineSpacingRule::Multiple => (rtf_measure(spacing, 240.0, "line spacing")?, true),
                LineSpacingRule::AtLeast => (rtf_measure(spacing, 15.0, "line spacing")?, false),
                LineSpacingRule::Exactly => (-rtf_measure(spacing, 15.0, "line spacing")?, false),
                _ => return Err(RtfError::UnsupportedFeature("line spacing rule")),
            };
            let _ = write!(self.output, "\\sl{value}\\slmult{}", i32::from(multiple));
            return Ok(());
        }
        if let Some(height) = format.line_height {
            if height.is_nan() || height == 0.0 {
                self.output.push_str("\\sl0\\slmult0");
                return Ok(());
            }
            if !height.is_finite() || height < 0.0 {
                return Err(RtfError::InvalidParameter("line height"));
            }
            let strategy = format.line_stacking_strategy.unwrap_or_default();
            if strategy == LineStackingStrategy::BaselineToBaseline {
                return Err(RtfError::UnsupportedFeature(
                    "baseline-to-baseline line stacking",
                ));
            }
            let height = rtf_measure(height, 15.0, "line height")?;
            if height <= 0 {
                return Err(RtfError::InvalidParameter("line height"));
            }
            let _ = write!(
                self.output,
                "\\sl{}\\slmult0",
                if strategy == LineStackingStrategy::BlockLineHeight {
                    -height
                } else {
                    height
                }
            );
        }
        Ok(())
    }
    fn inlines(&mut self, inlines: &[InlineSnapshot]) -> Result<(), RtfError> {
        for inline in inlines {
            self.output.push('{');
            let start = self.output.len();
            match &inline.kind {
                InlineSnapshotKind::Bold(_) => self.output.push_str("\\b"),
                InlineSnapshotKind::Italic(_) => self.output.push_str("\\i"),
                InlineSnapshotKind::Underline(_) => self.output.push_str("\\ul"),
                _ => {}
            }
            self.format(&inline.format)?;
            if self.output.len() > start {
                self.output.push(' ');
            }
            match &inline.kind {
                InlineSnapshotKind::Run(text) => escape(text, &mut self.output),
                InlineSnapshotKind::LineBreak => self.output.push_str("\\line "),
                InlineSnapshotKind::Span(children)
                | InlineSnapshotKind::Bold(children)
                | InlineSnapshotKind::Italic(children)
                | InlineSnapshotKind::Underline(children) => self.inlines(children)?,
                InlineSnapshotKind::Hyperlink {
                    inlines,
                    navigate_uri,
                    target_name,
                    ..
                } => {
                    if let Some(uri) = navigate_uri {
                        if uri.contains('"') {
                            return Err(RtfError::InvalidParameter("hyperlink URI"));
                        }
                        self.output.push_str("{\\field{\\*\\fldinst HYPERLINK \"");
                        escape(uri, &mut self.output);
                        self.output.push('"');
                        if let Some(target) = target_name {
                            if target.contains('"') {
                                return Err(RtfError::InvalidParameter("hyperlink target"));
                            }
                            self.output.push_str(" \\\\t \"");
                            escape(target, &mut self.output);
                            self.output.push('"');
                        }
                        self.output.push_str("}{\\fldrslt ");
                        self.inlines(inlines)?;
                        self.output.push_str("}}");
                    } else {
                        self.inlines(inlines)?;
                    }
                }
                InlineSnapshotKind::Embedded(_) => {
                    return Err(RtfError::UnsupportedFeature("embedded controls"));
                }
            }
            self.output.push('}');
        }
        Ok(())
    }
}

fn rtf_language(number: Option<i32>) -> Result<CultureInfo, RtfError> {
    let identifier = number
        .filter(|value| (0..=65535).contains(value))
        .ok_or(RtfError::InvalidParameter("language identifier"))?;
    if identifier == 1024 {
        return Ok(CultureInfo::invariant_culture());
    }
    CultureInfo::get_culture_info_by_lcid(identifier).ok_or(RtfError::UnsupportedFeature(
        "unknown Windows language identifier",
    ))
}

fn rtf_measure(value: f64, scale: f64, parameter: &'static str) -> Result<i32, RtfError> {
    let value = (value * scale).round();
    if !value.is_finite() || value < i32::MIN as f64 || value > i32::MAX as f64 {
        return Err(RtfError::InvalidParameter(parameter));
    }
    Ok(value as i32)
}

fn escape(text: &str, output: &mut String) {
    for unit in text.encode_utf16() {
        match unit {
            0x5c | 0x7b | 0x7d => {
                output.push('\\');
                output.push(unit as u8 as char);
            }
            0x09 => output.push_str("\\tab "),
            0x0a | 0x2028 => output.push_str("\\line "),
            0x20..=0x7e => output.push(unit as u8 as char),
            _ => {
                let _ = write!(output, "\\u{}?", unit as i16);
            }
        }
    }
}
