//! Editable topology over the serialized SVG-style path command stream.
//!
//! Point identities are command offsets, so pointer-frequency coordinate edits
//! preserve identity without a second, synchronized vector representation.
//! Inserting/deleting commands or converting an implicit closing edge changes
//! offsets; clients must rebuild their selection after structural edits.

use crate::{Node, NodeKind, PathSegment, Point, Rect, Transform, VectorPath, GEOMETRY_EPSILON};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathPointKind {
    Anchor,
    Control1,
    Control2,
    QuadraticControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathPointId {
    pub segment: usize,
    pub kind: PathPointKind,
}

impl PathPointId {
    pub const fn anchor(segment: usize) -> Self {
        Self {
            segment,
            kind: PathPointKind::Anchor,
        }
    }
}

/// All positions are node-local. A quadratic control influences both endpoints;
/// `secondary_anchor` supplies its second guide line without duplicate handles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathEditPoint {
    pub id: PathPointId,
    pub position: Point,
    pub anchor: Point,
    pub anchor_id: PathPointId,
    pub secondary_anchor: Option<Point>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathSegmentHit {
    pub segment: usize,
    pub t: f64,
    pub position: Point,
    pub distance: f64,
}

/// Geometric conversion of the two tangents at an anchor. Corner retracts its
/// handles; Smooth aligns them while retaining their individual lengths;
/// Symmetric additionally makes their lengths equal. Subsequent handle edits
/// remain independent: these operations do not introduce hidden node metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchorMode {
    Corner,
    Smooth,
    Symmetric,
}

#[derive(Clone, Copy)]
struct Curve {
    points: [Point; 4],
    degree: usize,
}

impl Curve {
    fn line(start: Point, end: Point) -> Self {
        Self {
            points: [start, end, end, end],
            degree: 1,
        }
    }

    fn from_segment(start: Point, segment: &PathSegment) -> Option<Self> {
        Some(match *segment {
            PathSegment::LineTo { point } => Self::line(start, point),
            PathSegment::QuadraticTo { control, point } => Self {
                points: [start, control, point, point],
                degree: 2,
            },
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => Self {
                points: [start, control1, control2, point],
                degree: 3,
            },
            _ => return None,
        })
    }

    fn evaluate(self, t: f64) -> Point {
        let mut points = self.points;
        for level in (1..=self.degree).rev() {
            for i in 0..level {
                points[i] = interpolate(points[i], points[i + 1], t);
            }
        }
        points[0]
    }

    fn power_coefficients(self) -> [Point; 4] {
        let p = self.points;
        let d0 = p[1] - p[0];
        let d1 = p[2] - p[1];
        let d2 = p[3] - p[2];
        match self.degree {
            1 => [p[0], d0, Point::default(), Point::default()],
            2 => [p[0], d0 * 2.0, d1 - d0, Point::default()],
            _ => [p[0], d0 * 3.0, (d1 - d0) * 3.0, d2 - d1 * 2.0 + d0],
        }
    }

    fn as_cubic(self) -> (Point, Point, Point) {
        match self.degree {
            1 => (
                interpolate(self.points[0], self.points[1], 1.0 / 3.0),
                interpolate(self.points[0], self.points[1], 2.0 / 3.0),
                self.points[1],
            ),
            2 => (
                interpolate(self.points[0], self.points[1], 2.0 / 3.0),
                interpolate(self.points[2], self.points[1], 2.0 / 3.0),
                self.points[2],
            ),
            _ => (self.points[1], self.points[2], self.points[3]),
        }
    }

    fn hull_bounds(self) -> Rect {
        self.points[..=self.degree]
            .iter()
            .map(|p| Rect::new(p.x, p.y, 0.0, 0.0))
            .reduce(Rect::union)
            .unwrap_or_default()
    }

    fn nearest(self, query: Point) -> (f64, Point, f64) {
        // Normalize the polynomial before multiplication to retain precision
        // for translated artwork and avoid squaring document-scale values.
        let origin = self.points[0];
        let scale = self.points[..=self.degree]
            .iter()
            .chain(std::iter::once(&query))
            .map(|p| (p.x - origin.x).abs().max((p.y - origin.y).abs()))
            .fold(0.0, f64::max);
        if scale <= f64::MIN_POSITIVE || !scale.is_finite() {
            return (0.0, origin, origin.distance(query));
        }
        let normalized = Self {
            points: self.points.map(|p| (p - origin) * (1.0 / scale)),
            degree: self.degree,
        };
        let mut coefficients = normalized.power_coefficients();
        coefficients[0] = coefficients[0] - (query - origin) * (1.0 / scale);
        let mut stationary = [0.0; 6];
        for i in 0..=self.degree {
            for j in 1..=self.degree {
                stationary[i + j - 1] += dot(coefficients[i], coefficients[j]) * j as f64;
            }
        }
        let mut candidates = unit_polynomial_roots(&stationary[..self.degree * 2]);
        candidates.push(0.0);
        candidates.push(1.0);
        candidates
            .into_iter()
            .map(|t| {
                let position = self.evaluate(t);
                (t, position, position.distance(query))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2).then(a.0.total_cmp(&b.0)))
            .unwrap_or((0.0, origin, origin.distance(query)))
    }
}

#[derive(Clone, Copy)]
struct Edge {
    segment: usize,
    start_anchor: Option<usize>,
    end_anchor: usize,
    curve: Curve,
}

struct Topology {
    aliases: Vec<Option<usize>>,
    edges: Vec<Edge>,
}

impl Topology {
    fn new(path: &VectorPath) -> Option<Self> {
        let mut result = Self {
            aliases: vec![None; path.segments.len()],
            edges: Vec::with_capacity(path.segments.len()),
        };
        let mut current = Point::default();
        let mut current_anchor = None;
        let mut start = Point::default();
        let mut start_anchor = None;
        for (i, segment) in path.segments.iter().enumerate() {
            match *segment {
                PathSegment::MoveTo { point } => {
                    if !finite(point) {
                        return None;
                    }
                    result.aliases[i] = Some(i);
                    current = point;
                    current_anchor = Some(i);
                    start = point;
                    start_anchor = Some(i);
                }
                PathSegment::Close => {
                    if let Some(first) = start_anchor {
                        if let Some(last) = current_anchor {
                            // SVG circles and imported closed curves often end
                            // with an explicit endpoint at their MoveTo point.
                            // Treat those as one anchor, including both handles.
                            if last != first && current.distance(start) <= GEOMETRY_EPSILON {
                                result.aliases[last] = Some(first);
                            }
                        }
                        result.edges.push(Edge {
                            segment: i,
                            start_anchor: current_anchor,
                            end_anchor: first,
                            curve: Curve::line(current, start),
                        });
                        current = start;
                        current_anchor = Some(first);
                    }
                }
                _ => {
                    let curve = Curve::from_segment(current, segment)?;
                    if curve.points[..=curve.degree].iter().any(|p| !finite(*p)) {
                        return None;
                    }
                    result.aliases[i] = Some(i);
                    result.edges.push(Edge {
                        segment: i,
                        start_anchor: current_anchor,
                        end_anchor: i,
                        curve,
                    });
                    current = curve.points[curve.degree];
                    current_anchor = Some(i);
                }
            }
        }
        for i in 0..result.edges.len() {
            result.edges[i].start_anchor = result.edges[i]
                .start_anchor
                .and_then(|anchor| result.canonical(anchor));
            result.edges[i].end_anchor = result.canonical(result.edges[i].end_anchor)?;
        }
        Some(result)
    }

    fn canonical(&self, mut segment: usize) -> Option<usize> {
        loop {
            let next = self.aliases.get(segment).copied().flatten()?;
            if next == segment {
                return Some(next);
            }
            segment = next;
        }
    }
}

impl VectorPath {
    /// The single source of truth for anchor/control overlays and hit testing.
    /// Anchors precede handles. An explicit endpoint coincident with the start
    /// of a closed subpath is represented by that subpath's MoveTo anchor.
    pub fn edit_points(&self) -> Vec<PathEditPoint> {
        let Some(topology) = Topology::new(self) else {
            return Vec::new();
        };
        let mut points = Vec::with_capacity(self.segments.len() * 2);
        for (segment, command) in self.segments.iter().enumerate() {
            if topology.aliases[segment] != Some(segment) {
                continue;
            }
            if let Some(position) = endpoint(command) {
                let id = PathPointId::anchor(segment);
                points.push(PathEditPoint {
                    id,
                    position,
                    anchor: position,
                    anchor_id: id,
                    secondary_anchor: None,
                });
            }
        }
        for edge in topology.edges {
            let start_anchor = edge.start_anchor.unwrap_or(edge.end_anchor);
            let start = endpoint(&self.segments[start_anchor]).unwrap_or(edge.curve.points[0]);
            let end = endpoint(&self.segments[edge.end_anchor]).unwrap_or_default();
            let mut handle = |kind, position, anchor, anchor_segment, secondary_anchor| {
                points.push(PathEditPoint {
                    id: PathPointId {
                        segment: edge.segment,
                        kind,
                    },
                    position,
                    anchor,
                    anchor_id: PathPointId::anchor(anchor_segment),
                    secondary_anchor,
                });
            };
            match self.segments[edge.segment] {
                PathSegment::CubicTo {
                    control1, control2, ..
                } => {
                    handle(PathPointKind::Control1, control1, start, start_anchor, None);
                    handle(
                        PathPointKind::Control2,
                        control2,
                        end,
                        edge.end_anchor,
                        None,
                    );
                }
                PathSegment::QuadraticTo { control, .. } => handle(
                    PathPointKind::QuadraticControl,
                    control,
                    end,
                    edge.end_anchor,
                    Some(start),
                ),
                _ => {}
            }
        }
        points
    }

    pub fn point_position(&self, id: PathPointId) -> Option<Point> {
        let segment = self.segments.get(id.segment)?;
        match (id.kind, segment) {
            (PathPointKind::Anchor, _) => {
                let topology = Topology::new(self)?;
                endpoint(&self.segments[topology.canonical(id.segment)?])
            }
            (PathPointKind::Control1, PathSegment::CubicTo { control1, .. }) => Some(*control1),
            (PathPointKind::Control2, PathSegment::CubicTo { control2, .. }) => Some(*control2),
            (PathPointKind::QuadraticControl, PathSegment::QuadraticTo { control, .. }) => {
                Some(*control)
            }
            _ => None,
        }
    }

    /// Moves an anchor with all directly attached handles; a quadratic's shared
    /// handle moves once, including closed single-anchor loops. Control-point
    /// edits leave every anchor and unrelated handle unchanged. Returns false
    /// for invalid/stale IDs, non-finite positions, or an unchanged position.
    pub fn move_edit_point(&mut self, id: PathPointId, position: Point) -> bool {
        if !finite(position) {
            return false;
        }
        let Some(topology) = Topology::new(self) else {
            return false;
        };
        let Some(previous) = self.point_position(id) else {
            return false;
        };
        if previous == position {
            return false;
        }
        if id.kind != PathPointKind::Anchor {
            let Some(command) = self.segments.get_mut(id.segment) else {
                return false;
            };
            match (id.kind, command) {
                (PathPointKind::Control1, PathSegment::CubicTo { control1, .. }) => {
                    *control1 = position
                }
                (PathPointKind::Control2, PathSegment::CubicTo { control2, .. }) => {
                    *control2 = position
                }
                (PathPointKind::QuadraticControl, PathSegment::QuadraticTo { control, .. }) => {
                    *control = position
                }
                _ => return false,
            }
            return true;
        }
        let Some(anchor) = topology.canonical(id.segment) else {
            return false;
        };
        let delta = position - previous;
        // Validate the complete prospective edit before touching the command
        // stream. A finite pointer position can still overflow a distant handle.
        for edge in &topology.edges {
            let start = edge.start_anchor == Some(anchor);
            let end = edge.end_anchor == anchor;
            match self.segments[edge.segment] {
                PathSegment::CubicTo {
                    control1, control2, ..
                } => {
                    if (start && !finite(control1 + delta)) || (end && !finite(control2 + delta)) {
                        return false;
                    }
                }
                PathSegment::QuadraticTo { control, .. } => {
                    if (start || end) && !finite(control + delta) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        for (i, segment) in self.segments.iter_mut().enumerate() {
            if topology.canonical(i) == Some(anchor) {
                if let Some(point) = endpoint_mut(segment) {
                    *point = position;
                }
            }
        }
        for edge in topology.edges {
            let start = edge.start_anchor == Some(anchor);
            let end = edge.end_anchor == anchor;
            match &mut self.segments[edge.segment] {
                PathSegment::CubicTo {
                    control1, control2, ..
                } => {
                    if start {
                        *control1 = *control1 + delta;
                    }
                    if end {
                        *control2 = *control2 + delta;
                    }
                }
                PathSegment::QuadraticTo { control, .. } if start || end => {
                    *control = *control + delta
                }
                _ => {}
            }
        }
        true
    }

    /// Exact Bézier subdivision. The returned ID addresses the inserted anchor.
    /// Closing edges remain closed, and other subpaths/commands are untouched.
    pub fn split_segment(&mut self, segment: usize, t: f64) -> Option<PathPointId> {
        if !t.is_finite() || !(0.0 < t && t < 1.0) {
            return None;
        }
        let topology = Topology::new(self)?;
        let edge = topology.edges.iter().find(|edge| edge.segment == segment)?;
        let points = edge.curve.points;
        let mut replacement = Vec::with_capacity(2);
        match self.segments[segment] {
            PathSegment::LineTo { point } => {
                replacement.push(PathSegment::LineTo {
                    point: interpolate(points[0], point, t),
                });
                replacement.push(PathSegment::LineTo { point });
            }
            PathSegment::QuadraticTo { control, point } => {
                let a = interpolate(points[0], control, t);
                let b = interpolate(control, point, t);
                let middle = interpolate(a, b, t);
                replacement.push(PathSegment::QuadraticTo {
                    control: a,
                    point: middle,
                });
                replacement.push(PathSegment::QuadraticTo { control: b, point });
            }
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => {
                let a = interpolate(points[0], control1, t);
                let b = interpolate(control1, control2, t);
                let c = interpolate(control2, point, t);
                let d = interpolate(a, b, t);
                let e = interpolate(b, c, t);
                let middle = interpolate(d, e, t);
                replacement.push(PathSegment::CubicTo {
                    control1: a,
                    control2: d,
                    point: middle,
                });
                replacement.push(PathSegment::CubicTo {
                    control1: e,
                    control2: c,
                    point,
                });
            }
            PathSegment::Close => {
                replacement.push(PathSegment::LineTo {
                    point: edge.curve.evaluate(t),
                });
                replacement.push(PathSegment::Close);
            }
            _ => return None,
        }
        self.segments.splice(segment..=segment, replacement);
        Some(PathPointId::anchor(segment))
    }

    /// Finds the global closest point among every line, quadratic, cubic and
    /// implicit closing edge. Stationary-distance roots are isolated between
    /// derivative roots, retaining loops, inflections and tangential minima.
    pub fn nearest_segment(&self, point: Point) -> Option<PathSegmentHit> {
        if !finite(point) {
            return None;
        }
        let topology = Topology::new(self)?;
        let mut nearest: Option<PathSegmentHit> = None;
        for edge in topology.edges {
            if nearest.is_some_and(|best| {
                distance_to_rect(point, edge.curve.hull_bounds()) > best.distance
            }) {
                continue;
            }
            let (t, position, distance) = edge.curve.nearest(point);
            if !distance.is_finite() {
                continue;
            }
            if nearest.is_none_or(|best| distance < best.distance) {
                nearest = Some(PathSegmentHit {
                    segment: edge.segment,
                    t,
                    position,
                    distance,
                });
            }
        }
        nearest
    }

    /// Bounds from analytic derivative extrema, with no flattened point buffer.
    /// Isolated MoveTo anchors are included so single-anchor paths stay editable.
    pub fn tight_bounds(&self) -> Option<Rect> {
        let topology = Topology::new(self)?;
        let mut bounds: Option<Rect> = None;
        let mut include = |point: Point| {
            let rect = Rect::new(point.x, point.y, 0.0, 0.0);
            bounds = Some(bounds.map_or(rect, |previous| previous.union(rect)));
        };
        for command in &self.segments {
            if let Some(point) = endpoint(command) {
                include(point);
            }
        }
        for edge in topology.edges {
            include(edge.curve.points[0]);
            let power = edge.curve.power_coefficients();
            for axis in [false, true] {
                let mut derivative = [0.0; 3];
                for i in 1..=edge.curve.degree {
                    derivative[i - 1] = if axis { power[i].y } else { power[i].x } * i as f64;
                }
                for t in unit_polynomial_roots(&derivative[..edge.curve.degree]) {
                    include(edge.curve.evaluate(t));
                }
            }
        }
        bounds.filter(|bounds| bounds.is_finite())
    }

    /// Deletes an anchor and reconnects adjacent edges. Existing tangents at
    /// retained Bézier endpoints survive reconnection; removing a first/last
    /// open anchor drops its incident edge. Deleting a subpath's sole anchor
    /// removes that subpath. Other contours and the fill rule are preserved.
    pub fn delete_anchor(&mut self, segment: usize) -> bool {
        let Some(mut contour) = Contour::for_anchor(self, segment) else {
            return false;
        };
        let Some(index) = contour
            .anchor_commands
            .iter()
            .position(|id| *id == contour.selected)
        else {
            return false;
        };
        let count = contour.points.len();
        if count == 1 {
            self.segments.drain(contour.start..contour.end);
            return true;
        }
        if contour.closed {
            let previous = (index + count - 1) % count;
            let next = (index + 1) % count;
            contour.edges[previous] = join_edges(
                &contour.edges[previous],
                &contour.edges[index],
                contour.points[previous],
                contour.points[index],
                contour.points[next],
            );
            contour.edges.remove(index);
        } else if index == 0 {
            contour.edges.remove(0);
        } else if index == count - 1 {
            contour.edges.pop();
        } else {
            contour.edges[index - 1] = join_edges(
                &contour.edges[index - 1],
                &contour.edges[index],
                contour.points[index - 1],
                contour.points[index],
                contour.points[index + 1],
            );
            contour.edges.remove(index);
        }
        contour.points.remove(index);
        if contour.points.len() == 1 {
            contour.edges.clear();
            contour.closed = false;
        }
        let replacement = contour.commands();
        self.segments
            .splice(contour.start..contour.end, replacement);
        true
    }

    pub fn convert_anchor(&mut self, segment: usize, mode: AnchorMode) -> bool {
        let Some(mut contour) = Contour::for_anchor(self, segment) else {
            return false;
        };
        let Some(index) = contour
            .anchor_commands
            .iter()
            .position(|id| *id == contour.selected)
        else {
            return false;
        };
        let count = contour.points.len();
        if contour.edges.is_empty() {
            return false;
        }
        let incoming = if index > 0 {
            Some(index - 1)
        } else if contour.closed {
            Some(count - 1)
        } else {
            None
        };
        let outgoing = if index + 1 < count || contour.closed {
            Some(index)
        } else {
            None
        };
        let anchor = contour.points[index];
        let incoming_curve =
            incoming.and_then(|i| Curve::from_segment(contour.points[i], &contour.edges[i]));
        let outgoing_curve = outgoing.and_then(|i| Curve::from_segment(anchor, &contour.edges[i]));
        let previous = incoming_curve
            .map(|curve| curve.points[0])
            .unwrap_or(anchor);
        let next = outgoing_curve
            .map(|curve| curve.points[curve.degree])
            .unwrap_or(anchor);
        let in_control = incoming_curve
            .map(|curve| curve.as_cubic().1)
            .unwrap_or(anchor);
        let out_control = outgoing_curve
            .map(|curve| curve.as_cubic().0)
            .unwrap_or(anchor);
        let direction = unit(next - previous)
            .or_else(|| unit(out_control - anchor))
            .or_else(|| unit(anchor - in_control))
            .unwrap_or(Point::new(1.0, 0.0));
        let mut in_length = anchor.distance(in_control);
        let mut out_length = anchor.distance(out_control);
        if mode != AnchorMode::Corner {
            if in_length <= GEOMETRY_EPSILON {
                in_length = anchor.distance(previous) / 3.0;
            }
            if out_length <= GEOMETRY_EPSILON {
                out_length = anchor.distance(next) / 3.0;
            }
            if mode == AnchorMode::Symmetric && incoming.is_some() && outgoing.is_some() {
                let length = in_length * 0.5 + out_length * 0.5;
                in_length = length;
                out_length = length;
            }
        } else {
            in_length = 0.0;
            out_length = 0.0;
        }
        if let Some(i) = incoming {
            let Some(curve) = Curve::from_segment(contour.points[i], &contour.edges[i]) else {
                return false;
            };
            if mode != AnchorMode::Corner || curve.degree > 1 {
                let (control1, _, point) = curve.as_cubic();
                contour.edges[i] = PathSegment::CubicTo {
                    control1,
                    control2: anchor - direction * in_length,
                    point,
                };
            }
        }
        if let Some(i) = outgoing {
            let Some(curve) = Curve::from_segment(anchor, &contour.edges[i]) else {
                return false;
            };
            if mode != AnchorMode::Corner || curve.degree > 1 {
                let (_, control2, point) = curve.as_cubic();
                contour.edges[i] = PathSegment::CubicTo {
                    control1: anchor + direction * out_length,
                    control2,
                    point,
                };
            }
        }
        let replacement = contour.commands();
        if self.segments[contour.start..contour.end] == replacement {
            return false;
        }
        self.segments
            .splice(contour.start..contour.end, replacement);
        true
    }
}

/// Rebase a vector to positive local bounds while keeping all path points at
/// precisely the same parent-space positions. Rotation, reflection and affine
/// residuals remain unchanged, including under transformed ancestor frames.
/// This is suitable for gesture updates: point command identities do not move.
pub fn normalize_vector_bounds(node: &mut Node) -> bool {
    if node.kind != NodeKind::Vector {
        return false;
    }
    let Some(bounds) = node.path.tight_bounds() else {
        return node.path.segments.is_empty();
    };
    let width = bounds.width.max(1.0);
    let height = bounds.height.max(1.0);
    let center_delta = Point::new((width - node.width) * 0.5, (height - node.height) * 0.5);
    let rigid = Transform::rotation(node.rotation)
        * Transform::scale(
            if node.flip_x { -1.0 } else { 1.0 },
            if node.flip_y { -1.0 } else { 1.0 },
        );
    let origin = node
        .local_transform()
        .apply_vector(Point::new(bounds.x, bounds.y));
    let new_position =
        Point::new(node.x, node.y) + origin + rigid.apply_vector(center_delta) - center_delta;
    if !finite(new_position) {
        return false;
    }
    let path = node
        .path
        .transformed(Transform::translation(-bounds.x, -bounds.y));
    if Topology::new(&path).is_none() {
        return false;
    }
    node.path = path;
    node.x = new_position.x;
    node.y = new_position.y;
    node.width = width;
    node.height = height;
    true
}

/// A contour used only for structural edits. Dragging and subdivision support
/// arbitrary SVG command streams. Anchor removal/conversion intentionally
/// decline malformed contours or branching commands after Close instead of
/// silently changing their winding/topology.
struct Contour {
    start: usize,
    end: usize,
    selected: usize,
    closed: bool,
    points: Vec<Point>,
    anchor_commands: Vec<usize>,
    edges: Vec<PathSegment>,
}

impl Contour {
    fn for_anchor(path: &VectorPath, segment: usize) -> Option<Self> {
        let topology = Topology::new(path)?;
        let selected = topology.canonical(segment)?;
        let start = (0..=selected)
            .rev()
            .find(|i| matches!(path.segments[*i], PathSegment::MoveTo { .. }))?;
        let end = ((start + 1)..path.segments.len())
            .find(|i| matches!(path.segments[*i], PathSegment::MoveTo { .. }))
            .unwrap_or(path.segments.len());
        let mut result = Self {
            start,
            end,
            selected,
            closed: matches!(path.segments.get(end - 1), Some(PathSegment::Close)),
            points: vec![endpoint(&path.segments[start])?],
            anchor_commands: vec![start],
            edges: Vec::with_capacity(end - start),
        };
        for i in start + 1..end {
            if matches!(path.segments[i], PathSegment::Close) {
                if i != end - 1 {
                    return None;
                }
                continue;
            }
            result.points.push(endpoint(&path.segments[i])?);
            result.anchor_commands.push(i);
            result.edges.push(path.segments[i].clone());
        }
        if result.closed {
            if result.points.len() > 1
                && result.points.last()?.distance(result.points[0]) <= GEOMETRY_EPSILON
            {
                result.points.pop();
                result.anchor_commands.pop();
                if let Some(last) = result.edges.last_mut().and_then(endpoint_mut) {
                    *last = result.points[0];
                }
            } else {
                result.edges.push(PathSegment::LineTo {
                    point: result.points[0],
                });
            }
        }
        Some(result)
    }

    fn commands(&self) -> Vec<PathSegment> {
        let mut commands = Vec::with_capacity(self.edges.len() + 2);
        let Some(first) = self.points.first() else {
            return commands;
        };
        commands.push(PathSegment::MoveTo { point: *first });
        for (i, edge) in self.edges.iter().enumerate() {
            if self.closed
                && i == self.edges.len() - 1
                && matches!(edge, PathSegment::LineTo { .. })
            {
                continue;
            }
            commands.push(edge.clone());
        }
        if self.closed {
            commands.push(PathSegment::Close);
        }
        commands
    }
}

fn join_edges(
    incoming: &PathSegment,
    outgoing: &PathSegment,
    previous: Point,
    removed: Point,
    next: Point,
) -> PathSegment {
    if matches!(incoming, PathSegment::LineTo { .. })
        && matches!(outgoing, PathSegment::LineTo { .. })
    {
        return PathSegment::LineTo { point: next };
    }
    let control1 = Curve::from_segment(previous, incoming)
        .map(|curve| curve.as_cubic().0)
        .unwrap_or(previous);
    let control2 = Curve::from_segment(removed, outgoing)
        .map(|curve| curve.as_cubic().1)
        .unwrap_or(next);
    PathSegment::CubicTo {
        control1,
        control2,
        point: next,
    }
}

fn endpoint(segment: &PathSegment) -> Option<Point> {
    match *segment {
        PathSegment::MoveTo { point }
        | PathSegment::LineTo { point }
        | PathSegment::QuadraticTo { point, .. }
        | PathSegment::CubicTo { point, .. } => Some(point),
        PathSegment::Close => None,
    }
}

fn endpoint_mut(segment: &mut PathSegment) -> Option<&mut Point> {
    match segment {
        PathSegment::MoveTo { point }
        | PathSegment::LineTo { point }
        | PathSegment::QuadraticTo { point, .. }
        | PathSegment::CubicTo { point, .. } => Some(point),
        PathSegment::Close => None,
    }
}

fn finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn interpolate(a: Point, b: Point, t: f64) -> Point {
    // Unlike a + (b-a)*t, this remains finite for opposite large coordinates.
    a * (1.0 - t) + b * t
}

fn dot(a: Point, b: Point) -> f64 {
    a.x * b.x + a.y * b.y
}

fn unit(point: Point) -> Option<Point> {
    let length = point.x.hypot(point.y);
    (length > GEOMETRY_EPSILON && length.is_finite()).then(|| point * (1.0 / length))
}

fn distance_to_rect(point: Point, rect: Rect) -> f64 {
    let x = (rect.x - point.x).max(0.0).max(point.x - rect.right());
    let y = (rect.y - point.y).max(0.0).max(point.y - rect.bottom());
    x.hypot(y)
}

/// All roots in [0,1] for degree <= 5, including repeated roots. Derivative
/// roots partition the domain into monotonic intervals; bracketed bisection
/// then cannot jump to another branch as an unconstrained Newton step can.
fn unit_polynomial_roots(coefficients: &[f64]) -> Vec<f64> {
    let scale = coefficients.iter().map(|c| c.abs()).fold(0.0, f64::max);
    if scale <= f64::MIN_POSITIVE || !scale.is_finite() {
        return Vec::new();
    }
    let mut normalized: Vec<_> = coefficients.iter().map(|c| c / scale).collect();
    while normalized.len() > 1 && normalized.last().is_some_and(|c| c.abs() <= 1e-15) {
        normalized.pop();
    }
    if normalized.len() <= 1 {
        return Vec::new();
    }
    if normalized.len() == 2 {
        let t = -normalized[0] / normalized[1];
        return if (0.0..=1.0).contains(&t) {
            vec![t]
        } else {
            Vec::new()
        };
    }
    let derivative: Vec<_> = normalized
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| *c * i as f64)
        .collect();
    let mut partitions = unit_polynomial_roots(&derivative);
    partitions.insert(0, 0.0);
    partitions.push(1.0);
    let evaluate = |t: f64| normalized.iter().rev().fold(0.0, |value, c| value * t + c);
    let tolerance = normalized.iter().map(|c| c.abs()).sum::<f64>() * 4e-14;
    let mut roots = Vec::with_capacity(normalized.len());
    for &t in &partitions {
        if evaluate(t).abs() <= tolerance {
            roots.push(t);
        }
    }
    for interval in partitions.windows(2) {
        let (mut left, mut right) = (interval[0], interval[1]);
        let mut left_value = evaluate(left);
        let right_value = evaluate(right);
        if left_value == 0.0
            || right_value == 0.0
            || left_value.is_sign_positive() == right_value.is_sign_positive()
        {
            continue;
        }
        for _ in 0..56 {
            let middle = (left + right) * 0.5;
            let value = evaluate(middle);
            if value == 0.0 {
                left = middle;
                right = middle;
                break;
            }
            if left_value.is_sign_positive() == value.is_sign_positive() {
                left = middle;
                left_value = value;
            } else {
                right = middle;
            }
        }
        roots.push((left + right) * 0.5);
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    roots
}
