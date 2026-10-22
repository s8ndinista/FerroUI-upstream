//! Editable network artwork expressed with public FerroUI XAML geometries,
//! native brushes and clips. Clipping never changes the brush's source bounds.
use super::*;
use std::fmt::Write;

pub(super) fn path_data(path: &core::VectorPath) -> String {
    let mut data = if path.fill_rule == core::FillRule::EvenOdd { "F0 " } else { "F1 " }.to_string();
    for segment in &path.segments {
        match segment {
            core::PathSegment::MoveTo { point } => { let _ = write!(data, "M{},{} ", point.x, point.y); }
            core::PathSegment::LineTo { point } => { let _ = write!(data, "L{},{} ", point.x, point.y); }
            core::PathSegment::CubicTo { control1, control2, point } => {
                let _ = write!(data, "C{},{} {},{} {},{} ", control1.x, control1.y, control2.x, control2.y, point.x, point.y);
            }
            core::PathSegment::QuadraticTo { control, point } => {
                let _ = write!(data, "Q{},{} {},{} ", control.x, control.y, point.x, point.y);
            }
            core::PathSegment::Close => data.push_str("Z "),
        }
    }
    data
}

fn path_resource(key: &str, path: &core::VectorPath, depth: usize, output: &mut String) {
    let pad = "  ".repeat(depth);
    let _ = writeln!(output, "{pad}<StreamGeometry x:Key=\"{key}\">{}</StreamGeometry>", path_data(path));
}

fn combine(key: &str, mode: &str, a: &str, b: &str, depth: usize, output: &mut String) {
    let pad = "  ".repeat(depth);
    let _ = writeln!(output, "{pad}<CombinedGeometry x:Key=\"{key}\" GeometryCombineMode=\"{mode}\" Geometry1=\"{{StaticResource {a}}}\" Geometry2=\"{{StaticResource {b}}}\"/>");
}

/// The reusable balanced cutter tree has bounded reference depth. Each suffix
/// uses only logarithmically many existing subtrees, without copying contours.
fn cut_tree(keys: &[String], prefix: &str, depth: usize, output: &mut String) -> String {
    if keys.len() == 1 { return keys[0].clone(); }
    let middle = keys.len() / 2;
    let left = cut_tree(&keys[..middle], &format!("{prefix}L"), depth, output);
    let right = cut_tree(&keys[middle..], &format!("{prefix}R"), depth, output);
    combine(prefix, "Union", &left, &right, depth, output);
    prefix.to_string()
}

fn suffix(keys: &[String], from: usize, prefix: &str, output: &mut Vec<String>) {
    if from >= keys.len() { return; }
    if from == 0 {
        output.push(if keys.len() == 1 { keys[0].clone() } else { prefix.to_string() });
        return;
    }
    let middle = keys.len() / 2;
    if from < middle {
        suffix(&keys[..middle], from, &format!("{prefix}L"), output);
        suffix(&keys[middle..], 0, &format!("{prefix}R"), output);
    } else {
        suffix(&keys[middle..], from - middle, &format!("{prefix}R"), output);
    }
}

fn fill_stack(path: &str, clip: Option<&str>, fills: &[Fill], depth: usize, output: &mut String) {
    let pad = "  ".repeat(depth);
    for fill in fills.iter().rev().filter(|fill| fill.visible && fill.opacity > 0.0) {
        let mut attributes = format!("Data=\"{{StaticResource {path}}}\" Stretch=\"None\"");
        if let Some(clip) = clip { let _ = write!(attributes, " Clip=\"{{StaticResource {clip}}}\""); }
        let mut properties = String::new();
        xaml_paint("Path", "Fill", &fill.paint, fill.opacity, depth + 1, &mut attributes, &mut properties);
        if properties.is_empty() {
            let _ = writeln!(output, "{pad}<Path {attributes}/>");
        } else {
            let _ = writeln!(output, "{pad}<Path {attributes}>\n{properties}{pad}</Path>");
        }
    }
}

pub(super) fn network_xaml(node: &Node, network: &core::VectorNetwork, depth: usize, output: &mut String) {
    let pad = "  ".repeat(depth);
    let transform = node.local_transform();
    // The complete editable affine transform includes center rotation, flips
    // and shear. Canvas.Left/Top must not apply its translation a second time.
    let _ = writeln!(output, "{pad}<Canvas Width=\"{}\" Height=\"{}\" IsVisible=\"{}\" RenderTransformOrigin=\"0%,0%\">",
        node.width, node.height, if node.visible { "True" } else { "False" });
    let _ = writeln!(output, "{pad}  <Canvas.RenderTransform><MatrixTransform Matrix=\"{},{},{},{},{},{}\"/></Canvas.RenderTransform>",
        transform.a, transform.b, transform.c, transform.d, transform.e, transform.f);
    if node.opacity != 1.0 {
        // A native opacity-mask scope composites the complete paint stack
        // before multiplying its alpha, including in the immediate renderer.
        let _ = writeln!(output, "{pad}  <Canvas.OpacityMask><SolidColorBrush Color=\"#FFFFFFFF\" Opacity=\"{}\"/></Canvas.OpacityMask>", node.opacity);
    }
    let _ = writeln!(output, "{pad}  <Canvas.Resources>");
    let prefix = format!("Network{}", node.id);
    let full = format!("{prefix}Fill");
    let inherited = format!("{prefix}Inherited");
    let stroke = format!("{prefix}Stroke");
    path_resource(&full, &network.fill_path(), depth + 2, output);
    path_resource(&inherited, &network.inherited_fill_path(), depth + 2, output);
    path_resource(&stroke, &network.stroke_path(), depth + 2, output);
    let regions: Vec<_> = network.regions.iter().filter_map(|(&id, region)| {
        Some((format!("{prefix}Region{id}"), network.region_fill_path(id)?, region.fills.as_ref()?))
    }).collect();
    for (key, path, _) in &regions { path_resource(key, path, depth + 2, output); }
    let keys: Vec<_> = regions.iter().map(|(key, _, _)| key.clone()).collect();
    let mut clips = vec![None; regions.len()];
    let inherited_clip = if keys.is_empty() { None } else {
        let cut_prefix = format!("{prefix}Cuts");
        let all = cut_tree(&keys, &cut_prefix, depth + 2, output);
        let clip = format!("{prefix}InheritedClip");
        combine(&clip, "Exclude", &inherited, &all, depth + 2, output);
        for index in 0..regions.len().saturating_sub(1) {
            let mut later = Vec::new();
            suffix(&keys, index + 1, &cut_prefix, &mut later);
            let cutter = cut_tree(&later, &format!("{prefix}Later{index}"), depth + 2, output);
            let clip = format!("{prefix}Clip{index}");
            combine(&clip, "Exclude", &keys[index], &cutter, depth + 2, output);
            clips[index] = Some(clip);
        }
        Some(clip)
    };
    let _ = writeln!(output, "{pad}  </Canvas.Resources>");
    // Legacy networks already draw the full fill path. Clipping that path to
    // itself would apply edge coverage twice and change translucent curves.
    fill_stack(&full, inherited_clip.as_deref(), &node.fills, depth + 1, output);
    for (index, (key, _, fills)) in regions.iter().enumerate() {
        fill_stack(key, clips[index].as_deref(), fills, depth + 1, output);
    }
    // Shared edges belong to the network's stroke graph exactly once. They
    // must not be stroked independently around each painted face.
    for paint in node.strokes.iter().rev().filter(|stroke| stroke.visible && stroke.width > 0.0 && stroke.opacity > 0.0) {
        let cap = match paint.cap { core::StrokeCap::Butt | core::StrokeCap::Arrow => "Flat", core::StrokeCap::Round => "Round", core::StrokeCap::Square => "Square" };
        let join = match paint.join { core::StrokeJoin::Miter => "Miter", core::StrokeJoin::Round => "Round", core::StrokeJoin::Bevel => "Bevel" };
        let mut attributes = format!("Data=\"{{StaticResource {stroke}}}\" Stretch=\"None\" StrokeThickness=\"{}\" StrokeLineCap=\"{cap}\" StrokeJoin=\"{join}\"", paint.width);
        if !paint.dash.is_empty() {
            let dash = paint.dash.iter().filter(|value| value.is_finite() && **value > 0.0)
                .map(|value| (value / paint.width).to_string()).collect::<Vec<_>>().join(",");
            if !dash.is_empty() { let _ = write!(attributes, " StrokeDashArray=\"{dash}\""); }
        }
        let mut properties = String::new();
        xaml_paint("Path", "Stroke", &paint.paint, paint.opacity, depth + 2, &mut attributes, &mut properties);
        if properties.is_empty() { let _ = writeln!(output, "{pad}  <Path {attributes}/>"); }
        else { let _ = writeln!(output, "{pad}  <Path {attributes}>\n{properties}{pad}  </Path>"); }
    }
    let _ = writeln!(output, "{pad}</Canvas>");
}

#[cfg(test)]
#[path = "vector_export_tests.rs"]
mod tests;
