use crate::*;
use std::collections::BTreeSet;
use std::fmt::Write;

#[derive(Clone, Debug)]
pub struct SvgExport {
    pub source: String,
    pub warnings: Vec<String>,
}

pub fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn number(value: f64) -> String {
    if !value.is_finite() {
        return "0".into();
    }
    let s = format!("{value:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.into()
    }
}

fn matrix(t: Transform) -> String {
    format!(
        "matrix({} {} {} {} {} {})",
        number(t.a),
        number(t.b),
        number(t.c),
        number(t.d),
        number(t.e),
        number(t.f)
    )
}

fn blend(mode: BlendMode) -> &'static str {
    match mode {
        BlendMode::Normal => "normal",
        BlendMode::Multiply => "multiply",
        BlendMode::Screen => "screen",
        BlendMode::Overlay => "overlay",
        BlendMode::Darken => "darken",
        BlendMode::Lighten => "lighten",
        BlendMode::ColorDodge => "color-dodge",
        BlendMode::ColorBurn => "color-burn",
        BlendMode::HardLight => "hard-light",
        BlendMode::SoftLight => "soft-light",
        BlendMode::Difference => "difference",
        BlendMode::Exclusion => "exclusion",
        BlendMode::Hue => "hue",
        BlendMode::Saturation => "saturation",
        BlendMode::Color => "color",
        BlendMode::Luminosity => "luminosity",
    }
}

fn paint_color(paint: &Paint, id: &str) -> String {
    match paint {
        Paint::Solid(c) => format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b),
        _ => format!("url(#{id})"),
    }
}
fn paint_opacity(paint: &Paint, opacity: f64) -> f64 {
    opacity.clamp(0.0, 1.0)
        * match paint {
            Paint::Solid(c) => c.a as f64 / 255.0,
            _ => 1.0,
        }
}

fn paint_definition(paint: &Paint, id: &str, node: &Node, out: &mut String) {
    match paint {
        Paint::Solid(_) => {}
        Paint::LinearGradient { start, end, stops } => {
            let _ = write!(
                out,
                r#"<linearGradient id="{id}" x1="{}" y1="{}" x2="{}" y2="{}">"#,
                number(start.x),
                number(start.y),
                number(end.x),
                number(end.y)
            );
            stops_definition(stops, out);
            out.push_str("</linearGradient>");
        }
        Paint::RadialGradient {
            center,
            radius,
            stops,
        } => {
            let _ = write!(
                out,
                r#"<radialGradient id="{id}" cx="{}" cy="{}" r="{}">"#,
                number(center.x),
                number(center.y),
                number(*radius)
            );
            stops_definition(stops, out);
            out.push_str("</radialGradient>");
        }
        Paint::Image { href, scale } => {
            let mode = match scale {
                ImageScale::Fill => "xMidYMid slice",
                ImageScale::Fit => "xMidYMid meet",
                _ => "none",
            };
            let _ = write!(
                out,
                r#"<pattern id="{id}" width="1" height="1" patternContentUnits="objectBoundingBox"><image href="{}" width="1" height="1" preserveAspectRatio="{mode}"/></pattern>"#,
                escape_xml(&safe_image_href(href))
            );
            let _ = node;
        }
    }
}
fn safe_image_href(href: &str) -> String {
    let lower = href.trim().to_ascii_lowercase();
    if lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("data:image/png;")
        || lower.starts_with("data:image/jpeg;")
        || lower.starts_with("data:image/webp;")
        || lower.starts_with("data:image/gif;")
    {
        href.trim().into()
    } else {
        String::new()
    }
}
fn stops_definition(stops: &[GradientStop], out: &mut String) {
    let mut sorted = stops.to_vec();
    sorted.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    for stop in sorted {
        let c = stop.color;
        let _ = write!(
            out,
            r##"<stop offset="{}" stop-color="#{:02X}{:02X}{:02X}" stop-opacity="{}"/>"##,
            number(stop.offset.clamp(0.0, 1.0)),
            c.r,
            c.g,
            c.b,
            number(c.a as f64 / 255.0)
        );
    }
}

fn path_data(path: &VectorPath, t: Transform) -> String {
    let mut out = String::new();
    for segment in &path.segments {
        match *segment {
            PathSegment::MoveTo { point } => {
                let p = t.apply(point);
                let _ = write!(out, "M{} {} ", number(p.x), number(p.y));
            }
            PathSegment::LineTo { point } => {
                let p = t.apply(point);
                let _ = write!(out, "L{} {} ", number(p.x), number(p.y));
            }
            PathSegment::QuadraticTo { control, point } => {
                let c = t.apply(control);
                let p = t.apply(point);
                let _ = write!(
                    out,
                    "Q{} {} {} {} ",
                    number(c.x),
                    number(c.y),
                    number(p.x),
                    number(p.y)
                );
            }
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => {
                let a = t.apply(control1);
                let b = t.apply(control2);
                let p = t.apply(point);
                let _ = write!(
                    out,
                    "C{} {} {} {} {} {} ",
                    number(a.x),
                    number(a.y),
                    number(b.x),
                    number(b.y),
                    number(p.x),
                    number(p.y)
                );
            }
            PathSegment::Close => out.push_str("Z "),
        }
    }
    out
}

pub fn outline_path(node: &Node) -> VectorPath {
    if node.kind == NodeKind::Vector {
        return node.vector_network.as_ref().map(|network| network.fill_path()).unwrap_or_else(|| node.path.clone());
    }
    if node.kind == NodeKind::Polygon || node.kind == NodeKind::Star {
        return VectorPath::polygon(&node.polygon_points(), true);
    }
    if node.kind == NodeKind::Line {
        return VectorPath::polygon(
            &[Point::default(), Point::new(node.width, node.height)],
            false,
        );
    }
    let w = node.width;
    let h = node.height;
    let k = 0.552_284_749_830_793_6;
    if node.kind == NodeKind::Ellipse {
        let rx = w / 2.0;
        let ry = h / 2.0;
        return VectorPath {
            fill_rule: FillRule::NonZero,
            anchor_modes: Default::default(),
            segments: vec![
                PathSegment::MoveTo {
                    point: Point::new(w, ry),
                },
                PathSegment::CubicTo {
                    control1: Point::new(w, ry + k * ry),
                    control2: Point::new(rx + k * rx, h),
                    point: Point::new(rx, h),
                },
                PathSegment::CubicTo {
                    control1: Point::new(rx - k * rx, h),
                    control2: Point::new(0.0, ry + k * ry),
                    point: Point::new(0.0, ry),
                },
                PathSegment::CubicTo {
                    control1: Point::new(0.0, ry - k * ry),
                    control2: Point::new(rx - k * rx, 0.0),
                    point: Point::new(rx, 0.0),
                },
                PathSegment::CubicTo {
                    control1: Point::new(rx + k * rx, 0.0),
                    control2: Point::new(w, ry - k * ry),
                    point: Point::new(w, ry),
                },
                PathSegment::Close,
            ],
        };
    }
    let radii = node
        .corner_radii
        .unwrap_or([node.corner_radius; 4])
        .map(|r| r.max(0.0).min(w / 2.0).min(h / 2.0));
    let [tl, tr, br, bl] = radii;
    VectorPath {
        fill_rule: FillRule::NonZero,
            anchor_modes: Default::default(),
        segments: vec![
            PathSegment::MoveTo {
                point: Point::new(tl, 0.0),
            },
            PathSegment::LineTo {
                point: Point::new(w - tr, 0.0),
            },
            PathSegment::CubicTo {
                control1: Point::new(w - tr + k * tr, 0.0),
                control2: Point::new(w, tr - k * tr),
                point: Point::new(w, tr),
            },
            PathSegment::LineTo {
                point: Point::new(w, h - br),
            },
            PathSegment::CubicTo {
                control1: Point::new(w, h - br + k * br),
                control2: Point::new(w - br + k * br, h),
                point: Point::new(w - br, h),
            },
            PathSegment::LineTo {
                point: Point::new(bl, h),
            },
            PathSegment::CubicTo {
                control1: Point::new(bl - k * bl, h),
                control2: Point::new(0.0, h - bl + k * bl),
                point: Point::new(0.0, h - bl),
            },
            PathSegment::LineTo {
                point: Point::new(0.0, tl),
            },
            PathSegment::CubicTo {
                control1: Point::new(0.0, tl - k * tl),
                control2: Point::new(tl - k * tl, 0.0),
                point: Point::new(tl, 0.0),
            },
            PathSegment::Close,
        ],
    }
}

fn node_shape(node: &Node, attributes: &str) -> String {
    match node.kind {
        NodeKind::Text => text_shape(node, attributes),
        _ => format!(
            r#"<path d="{}" fill-rule="{}" {attributes}/>"#,
            path_data(&outline_path(node), Transform::IDENTITY),
            if node.vector_network.as_ref().map(|g| g.fill_rule).unwrap_or(node.path.fill_rule) == FillRule::EvenOdd {
                "evenodd"
            } else {
                "nonzero"
            }
        ),
    }
}

fn stroke_shape(node: &Node, attributes: &str) -> String {
    if let Some(network) = &node.vector_network {
        format!(r#"<path d="{}" {attributes}/>"#, path_data(&network.stroke_path(), Transform::IDENTITY))
    } else { node_shape(node, attributes) }
}

fn text_shape(node: &Node, attributes: &str) -> String {
    let anchor = match node.text_align {
        TextAlign::Center => "middle",
        TextAlign::Right => "end",
        _ => "start",
    };
    let x = match node.text_align {
        TextAlign::Center => node.width / 2.0,
        TextAlign::Right => node.width,
        _ => 0.0,
    };
    let lines: Vec<_> = node.text.split('\n').collect();
    let line_height = node.font_size * node.line_height;
    let text_height = node.font_size + line_height * lines.len().saturating_sub(1) as f64;
    let top = match node.text_vertical_align {
        TextVerticalAlign::Top => 0.0,
        TextVerticalAlign::Center => (node.height - text_height).max(0.0) / 2.0,
        TextVerticalAlign::Bottom => (node.height - text_height).max(0.0),
    };
    let decoration = match node.text_decoration {
        TextDecoration::None => "none",
        TextDecoration::Underline => "underline",
        TextDecoration::Strikethrough => "line-through",
    };
    let mut out = format!(
        r#"<text font-family="{}" font-size="{}" font-weight="{}" font-style="{}" text-anchor="{anchor}" letter-spacing="{}" text-decoration="{decoration}" xml:space="preserve" {attributes}>"#,
        escape_xml(&node.font_family),
        number(node.font_size),
        node.font_weight,
        if node.italic { "italic" } else { "normal" },
        number(node.letter_spacing)
    );
    let mut start = 0;
    for (i, line) in lines.iter().enumerate() {
        let _ = write!(
            out,
            r#"<tspan x="{}" y="{}">"#,
            number(x),
            number(top + node.font_size + i as f64 * line_height)
        );
        let end = start + line.len();
        let mut boundaries = vec![start, end];
        for run in &node.text_runs {
            if run.start < end && run.end > start {
                boundaries.push(run.start.max(start));
                boundaries.push(run.end.min(end));
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        if boundaries.len() == 1 {
            out.push_str("&#8203;")
        }
        for pair in boundaries.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if !node.text.is_char_boundary(a) || !node.text.is_char_boundary(b) {
                continue;
            }
            let run = node
                .text_runs
                .iter()
                .rev()
                .find(|run| run.start <= a && run.end >= b);
            if let Some(run) = run {
                out.push_str("<tspan");
                if let Some(family) = &run.font_family {
                    let _ = write!(out, r#" font-family="{}""#, escape_xml(family));
                }
                if let Some(size) = run.font_size {
                    let _ = write!(out, r#" font-size="{}""#, number(size));
                }
                if let Some(weight) = run.font_weight {
                    let _ = write!(out, r#" font-weight="{weight}""#);
                }
                if let Some(c) = run.color {
                    let _ = write!(
                        out,
                        r##" fill="#{:02X}{:02X}{:02X}" fill-opacity="{}""##,
                        c.r,
                        c.g,
                        c.b,
                        number(c.a as f64 / 255.0)
                    );
                }
                if let Some(italic) = run.italic {
                    let _ = write!(
                        out,
                        r#" font-style="{}""#,
                        if italic { "italic" } else { "normal" }
                    );
                }
                if let Some(decoration) = run.decoration {
                    let _ = write!(
                        out,
                        r#" text-decoration="{}""#,
                        match decoration {
                            TextDecoration::None => "none",
                            TextDecoration::Underline => "underline",
                            TextDecoration::Strikethrough => "line-through",
                        }
                    );
                }
                out.push('>');
                out.push_str(&escape_xml(&node.text[a..b]));
                out.push_str("</tspan>");
            } else {
                out.push_str(&escape_xml(&node.text[a..b]));
            }
        }
        out.push_str("</tspan>");
        start = end + 1;
    }
    out.push_str("</text>");
    out
}

impl Document {
    pub fn export_svg(&self, page: NodeId, selection: &[NodeId]) -> String {
        self.export_svg_report(page, selection).source
    }
    pub fn export_svg_report(&self, page: NodeId, selection: &[NodeId]) -> SvgExport {
        let selection_set: BTreeSet<_> = selection.iter().copied().collect();
        let roots: Vec<_> = if selection.is_empty() {
            self.page(page)
                .map(|p| p.children.clone())
                .unwrap_or_default()
        } else {
            selection
                .iter()
                .filter(|id| {
                    self.nodes.contains_key(id)
                        && !selection_set
                            .iter()
                            .any(|other| other != *id && self.is_ancestor(*other, **id))
                })
                .copied()
                .collect()
        };
        let bounds = roots
            .iter()
            .filter_map(|id| self.world_bounds(*id))
            .reduce(|a, b| a.union(b))
            .unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0));
        let mut defs = String::new();
        let mut body = String::new();
        let mut warnings = Vec::new();
        for id in roots {
            self.export_svg_node(id, true, &mut defs, &mut body, &mut warnings, 0);
        }
        let source = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="{}" viewBox="{} {} {} {}" role="img" aria-label="{}"><title>{}</title><defs>{}</defs>{}</svg>"#,
            number(bounds.width.max(1.0)),
            number(bounds.height.max(1.0)),
            number(bounds.x),
            number(bounds.y),
            number(bounds.width.max(1.0)),
            number(bounds.height.max(1.0)),
            escape_xml(&self.name),
            escape_xml(&self.name),
            defs,
            body
        );
        SvgExport { source, warnings }
    }

    fn export_svg_node(
        &self,
        id: NodeId,
        root: bool,
        defs: &mut String,
        body: &mut String,
        warnings: &mut Vec<String>,
        depth: usize,
    ) {
        if depth >= MAX_DOCUMENT_DEPTH {
            return;
        }
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        if !node.visible {
            return;
        }
        let transform = if root {
            self.world_transform(id).unwrap_or_default()
        } else {
            node.local_transform()
        };
        let _ = write!(
            body,
            r#"<g id="node-{id}" data-name="{}" transform="{}" opacity="{}" style="mix-blend-mode:{}">"#,
            escape_xml(&node.name),
            matrix(transform),
            number(node.opacity),
            blend(node.blend_mode)
        );
        let _ = write!(body, "<title>{}</title>", escape_xml(&node.name));
        let effects: Vec<_> = node.effects.iter().filter(|e| e.visible).collect();
        let has_filter = effects.iter().any(|e| e.kind != EffectKind::BackgroundBlur);
        if has_filter {
            let _ = write!(
                defs,
                r#"<filter id="effects-{id}" x="-100%" y="-100%" width="300%" height="300%" color-interpolation-filters="sRGB">"#
            );
            for (i, effect) in effects.iter().enumerate() {
                let c = effect.color;
                match effect.kind {
                    EffectKind::DropShadow => {
                        let _ = write!(
                            defs,
                            r##"<feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="#{:02X}{:02X}{:02X}" flood-opacity="{}"/>"##,
                            number(effect.offset.x),
                            number(effect.offset.y),
                            number(effect.radius / 2.0),
                            c.r,
                            c.g,
                            c.b,
                            number(c.a as f64 / 255.0)
                        );
                    }
                    EffectKind::InnerShadow => {
                        let _ = write!(
                            defs,
                            r##"<feGaussianBlur in="SourceAlpha" stdDeviation="{}" result="blur-{id}-{i}"/><feOffset dx="{}" dy="{}" result="offset-{id}-{i}"/><feComposite in="SourceAlpha" in2="offset-{id}-{i}" operator="out" result="inner-{id}-{i}"/><feFlood flood-color="#{:02X}{:02X}{:02X}" flood-opacity="{}"/><feComposite in2="inner-{id}-{i}" operator="in"/><feComposite in2="SourceGraphic" operator="over"/>"##,
                            number(effect.radius / 2.0),
                            number(effect.offset.x),
                            number(effect.offset.y),
                            c.r,
                            c.g,
                            c.b,
                            number(c.a as f64 / 255.0)
                        );
                    }
                    EffectKind::LayerBlur => {
                        let _ = write!(
                            defs,
                            r#"<feGaussianBlur stdDeviation="{}"/>"#,
                            number(effect.radius / 2.0)
                        );
                    }
                    EffectKind::BackgroundBlur => {}
                }
            }
            defs.push_str("</filter>");
            let _ = write!(body, r#"<g filter="url(#effects-{id})">"#);
        }
        if effects.iter().any(|e| e.kind == EffectKind::BackgroundBlur) {
            warnings.push(format!("Layer {}: background blur needs a composited backdrop; exported without backdrop blur",node.name));
        }
        let shape = node_shape(node, "");
        if node.clip_content {
            let _ = write!(defs, r#"<clipPath id="clip-{id}">{shape}</clipPath>"#);
            let _ = write!(body, r#"<g clip-path="url(#clip-{id})">"#);
        }
        if node.kind == NodeKind::Boolean {
            self.export_boolean(node, defs, body, warnings);
        } else {
            for (i, fill) in node.fills.iter().enumerate().filter(|(_, f)| f.visible) {
                let paint_id = format!("fill-{id}-{i}");
                paint_definition(&fill.paint, &paint_id, node, defs);
                body.push_str(&node_shape(
                    node,
                    &format!(
                        r#"fill="{}" fill-opacity="{}" stroke="none""#,
                        paint_color(&fill.paint, &paint_id),
                        number(paint_opacity(&fill.paint, fill.opacity))
                    ),
                ));
            }
            if node.kind == NodeKind::Text && node.fills.is_empty() {
                body.push_str(&node_shape(node, r#"fill="none""#));
            }
            for child in &node.children {
                self.export_svg_node(*child, false, defs, body, warnings, depth + 1);
            }
            for (i, stroke) in node
                .strokes
                .iter()
                .enumerate()
                .filter(|(_, s)| s.visible && s.width > 0.0)
            {
                let paint_id = format!("stroke-{id}-{i}");
                paint_definition(&stroke.paint, &paint_id, node, defs);
                let cap = match stroke.cap {
                    StrokeCap::Round => "round",
                    StrokeCap::Square => "square",
                    _ => "butt",
                };
                let join = match stroke.join {
                    StrokeJoin::Miter => "miter",
                    StrokeJoin::Round => "round",
                    StrokeJoin::Bevel => "bevel",
                };
                let mut attributes = format!(
                    r#"fill="none" stroke="{}" stroke-opacity="{}" stroke-width="{}" stroke-linecap="{cap}" stroke-linejoin="{join}""#,
                    paint_color(&stroke.paint, &paint_id),
                    number(paint_opacity(&stroke.paint, stroke.opacity)),
                    number(
                        stroke.width
                            * if stroke.align == StrokeAlign::Center {
                                1.0
                            } else {
                                2.0
                            }
                    )
                );
                if !stroke.dash.is_empty() {
                    let _ = write!(
                        attributes,
                        r#" stroke-dasharray="{}""#,
                        stroke
                            .dash
                            .iter()
                            .map(|v| number(*v))
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                }
                if stroke.cap == StrokeCap::Arrow {
                    let _ = write!(
                        defs,
                        r#"<marker id="arrow-{id}-{i}" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 Z" fill="{}"/></marker>"#,
                        paint_color(&stroke.paint, &paint_id)
                    );
                    let _ = write!(attributes, r#" marker-end="url(#arrow-{id}-{i})""#);
                }
                match stroke.align {
                    StrokeAlign::Center => body.push_str(&stroke_shape(node, &attributes)),
                    StrokeAlign::Inside => {
                        let _ = write!(
                            defs,
                            r#"<clipPath id="stroke-clip-{id}-{i}">{shape}</clipPath>"#
                        );
                        let _ = write!(
                            body,
                            r#"<g clip-path="url(#stroke-clip-{id}-{i})">{}</g>"#,
                            stroke_shape(node, &attributes)
                        );
                    }
                    StrokeAlign::Outside => {
                        let extent = stroke.width * 3.0 + 2.0;
                        let _ = write!(
                            defs,
                            r#"<mask id="stroke-mask-{id}-{i}" maskUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}"><rect x="{}" y="{}" width="{}" height="{}" fill="white"/>{}</mask>"#,
                            number(-extent),
                            number(-extent),
                            number(node.width + 2.0 * extent),
                            number(node.height + 2.0 * extent),
                            number(-extent),
                            number(-extent),
                            number(node.width + 2.0 * extent),
                            number(node.height + 2.0 * extent),
                            node_shape(node, r#"fill="black""#)
                        );
                        let _ = write!(
                            body,
                            r#"<g mask="url(#stroke-mask-{id}-{i})">{}</g>"#,
                            stroke_shape(node, &attributes)
                        );
                    }
                }
            }
        }
        if node.clip_content {
            body.push_str("</g>")
        }
        if has_filter {
            body.push_str("</g>")
        }
        body.push_str("</g>");
    }

    fn export_boolean(
        &self,
        node: &Node,
        defs: &mut String,
        body: &mut String,
        warnings: &mut Vec<String>,
    ) {
        let id = node.id;
        let operands: Vec<_> = node
            .children
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .filter(|n| n.visible)
            .collect();
        let _ = write!(
            defs,
            r#"<mask id="boolean-{id}" maskUnits="userSpaceOnUse" x="-1" y="-1" width="{}" height="{}">"#,
            number(node.width + 2.0),
            number(node.height + 2.0)
        );
        if node.boolean_operation == BooleanOperation::Exclude {
            let paths = operands
                .iter()
                .map(|n| path_data(&outline_path(n), n.local_transform()))
                .collect::<Vec<_>>()
                .join(" ");
            let _ = write!(
                defs,
                r#"<path d="{paths}" fill="white" fill-rule="evenodd"/>"#
            );
        } else if node.boolean_operation == BooleanOperation::Intersect {
            for (i, n) in operands.iter().enumerate() {
                let path = path_data(&outline_path(n), n.local_transform());
                let _ = write!(
                    defs,
                    r#"<clipPath id="operand-{id}-{i}"><path d="{path}"/></clipPath><g clip-path="url(#operand-{id}-{i})">"#
                );
            }
            let _ = write!(
                defs,
                r#"<rect width="{}" height="{}" fill="white"/>"#,
                number(node.width),
                number(node.height)
            );
            for _ in &operands {
                defs.push_str("</g>")
            }
        } else {
            for (i, n) in operands.iter().enumerate() {
                let path = path_data(&outline_path(n), n.local_transform());
                let fill = if node.boolean_operation == BooleanOperation::Subtract && i > 0 {
                    "black"
                } else {
                    "white"
                };
                let _ = write!(defs, r#"<path d="{path}" fill="{fill}"/>"#);
            }
        }
        defs.push_str("</mask>");
        for (i, fill) in node.fills.iter().enumerate().filter(|(_, f)| f.visible) {
            let paint_id = format!("fill-{id}-{i}");
            paint_definition(&fill.paint, &paint_id, node, defs);
            let _ = write!(
                body,
                r#"<rect width="{}" height="{}" fill="{}" fill-opacity="{}" mask="url(#boolean-{id})"/>"#,
                number(node.width),
                number(node.height),
                paint_color(&fill.paint, &paint_id),
                number(paint_opacity(&fill.paint, fill.opacity))
            );
        }
        if operands.iter().any(|n| n.kind == NodeKind::Text) {
            warnings.push(format!("Boolean group {}: text operands use their layout bounds; convert text to glyph outlines for exact typography booleans",node.name));
        }
    }
}
