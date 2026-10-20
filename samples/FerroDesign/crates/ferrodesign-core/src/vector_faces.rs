//! Checked planar embedding and exact, edge-owned bounded faces.
//!
//! Face boundaries retain authored Bézier commands. Subdivision is used only
//! to certify that control hulls do not intersect, never to approximate output.
//! The operation deliberately declines ambiguous/touching geometry: it neither
//! invents junctions at crossings nor mutates the graph while discovering faces.

use super::*;
use std::fmt;

const MAX_EDGES: usize = 32_768;
const MAX_GEOMETRY_WORK: usize = 1_000_000;
const MAX_SUBDIVISION: u8 = 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkFaceError {
    InvalidNetwork(String),
    NotEditable,
    DegenerateEdge(VectorEdgeId),
    AmbiguousJunction(VectorVertexId),
    UnsplitIntersection(VectorEdgeId, VectorEdgeId),
    IndeterminateBoundary,
    ComplexityLimit,
    IdExhausted,
}

impl fmt::Display for NetworkFaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNetwork(reason) => write!(f, "Invalid vector network: {reason}"),
            Self::NotEditable => f.write_str("Select an unlocked vector network."),
            Self::DegenerateEdge(edge) => write!(f, "Edge {edge} has degenerate or singular geometry; remove or split it before finding faces."),
            Self::AmbiguousJunction(vertex) => write!(f, "Vertex {vertex} has coincident outgoing tangents; separate or split those edges before finding faces."),
            Self::UnsplitIntersection(a, b) => write!(f, "Edges {a} and {b} cross, overlap, touch without a shared vertex, or cannot be separated reliably. Split and weld their intersections before finding faces."),
            Self::IndeterminateBoundary => f.write_str("A face has an indeterminate orientation or repeated boundary; simplify its geometry before finding faces."),
            Self::ComplexityLimit => f.write_str("Face discovery exceeded its geometry work limit; split this network into smaller layers."),
            Self::IdExhausted => f.write_str("The vector network has exhausted its stable ID space."),
        }
    }
}

impl std::error::Error for NetworkFaceError {}

struct Work(usize);

impl Work {
    fn take(&mut self) -> Result<(), NetworkFaceError> {
        self.0 = self.0.checked_sub(1).ok_or(NetworkFaceError::ComplexityLimit)?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct EmbeddedEdge {
    id: VectorEdgeId,
    start: usize,
    end: usize,
    curve: Curve,
    bounds: Rect,
}

struct Boundary {
    contour: Vec<NetworkEdgeRef>,
    area: f64,
    bounds: Rect,
}

impl VectorNetwork {
    /// Finds every bounded face of a checked planar line/cubic embedding.
    /// Open branches are excluded from boundaries. Nested disconnected cycles
    /// become an outer face with oriented holes and separate inner faces.
    /// Unsplit intersections, coincident tangents and numerically unresolved
    /// curve contacts return an error without changing regions or allocating IDs.
    pub fn detect_faces(&self) -> Result<Vec<NetworkRegion>, NetworkFaceError> {
        self.validate().map_err(NetworkFaceError::InvalidNetwork)?;
        if self.edges.is_empty() {
            return Ok(Vec::new());
        }
        if self.edges.len() > MAX_EDGES {
            return Err(NetworkFaceError::ComplexityLimit);
        }
        let mut work = Work(MAX_GEOMETRY_WORK);
        let vertices: Vec<_> = self.vertices.keys().copied().collect();
        let vertex_index: BTreeMap<_, _> = vertices.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        // Normalize only the geometric certification. Original controls and
        // stable edge references remain authoritative in returned contours.
        let bounds = self.edges.keys().filter_map(|&id| self.edge_curve(id).map(Curve::hull_bounds))
            .reduce(Rect::union).ok_or(NetworkFaceError::IndeterminateBoundary)?;
        let scale = bounds.width.max(bounds.height);
        if !scale.is_finite() || scale <= 0.0 {
            return Err(NetworkFaceError::DegenerateEdge(*self.edges.keys().next().unwrap()));
        }
        let origin = Point::new(bounds.x, bounds.y);
        let mut edges = Vec::with_capacity(self.edges.len());
        let mut adjacency = vec![Vec::<usize>::new(); vertices.len()];
        for (&id, edge) in &self.edges {
            let mut curve = self.edge_curve(id).ok_or(NetworkFaceError::DegenerateEdge(id))?;
            curve.points = curve.points.map(|point| (point - origin) * (1.0 / scale));
            if curve.points[..=curve.degree].iter().any(|point| !finite(*point))
                || curve.points[..=curve.degree].iter().all(|point| *point == curve.points[0])
            {
                return Err(NetworkFaceError::DegenerateEdge(id));
            }
            certify_simple_curve(curve, id, &mut work)?;
            let index = edges.len();
            let start = vertex_index[&edge.start];
            let end = vertex_index[&edge.end];
            adjacency[start].push(index);
            adjacency[end].push(index);
            edges.push(EmbeddedEdge { id, start, end, curve, bounds: curve.hull_bounds() });
        }
        certify_embedding(&edges, &mut work)?;
        let bridges = find_bridges(&edges, &adjacency);
        let mut outgoing = vec![Vec::<usize>::new(); vertices.len()];
        for (index, edge) in edges.iter().enumerate() {
            if bridges[index] { continue; }
            outgoing[edge.start].push(index * 2);
            outgoing[edge.end].push(index * 2 + 1);
        }
        let mut angles = vec![0.0; edges.len() * 2];
        for (vertex, refs) in outgoing.iter_mut().enumerate() {
            for &reference in refs.iter() {
                let direction = outgoing_direction(edges[reference / 2].curve, reference % 2 == 1)
                    .ok_or(NetworkFaceError::DegenerateEdge(edges[reference / 2].id))?;
                angles[reference] = direction.y.atan2(direction.x);
            }
            refs.sort_by(|a, b| angles[*a].total_cmp(&angles[*b]).then(a.cmp(b)));
            if refs.len() > 1 {
                for (&a, &b) in refs.iter().zip(refs.iter().cycle().skip(1)) {
                    let delta = (angles[a] - angles[b]).abs();
                    if delta.min(std::f64::consts::TAU - delta) <= 128.0 * f64::EPSILON {
                        return Err(NetworkFaceError::AmbiguousJunction(vertices[vertex]));
                    }
                }
            }
        }
        let mut next = vec![usize::MAX; edges.len() * 2];
        for refs in &outgoing {
            for (index, &reference) in refs.iter().enumerate() {
                next[reference ^ 1] = refs[(index + refs.len() - 1) % refs.len()];
            }
        }
        let mut visited = vec![false; next.len()];
        let mut positive = Vec::<Boundary>::new();
        let mut negative = Vec::<Boundary>::new();
        for start in 0..next.len() {
            if visited[start] || next[start] == usize::MAX { continue; }
            let mut current = start;
            let mut contour = Vec::new();
            let mut seen_edges = BTreeSet::new();
            loop {
                if visited[current] {
                    if current != start { return Err(NetworkFaceError::IndeterminateBoundary); }
                    break;
                }
                visited[current] = true;
                let edge = edges[current / 2];
                if !seen_edges.insert(edge.id) { return Err(NetworkFaceError::IndeterminateBoundary); }
                contour.push(NetworkEdgeRef { edge: edge.id, reversed: current % 2 == 1 });
                current = next[current];
                if current == usize::MAX { return Err(NetworkFaceError::IndeterminateBoundary); }
            }
            canonicalize_contour(&mut contour);
            let first = &edges[edges.binary_search_by_key(&contour[0].edge, |edge| edge.id).unwrap()];
            let local_origin = if contour[0].reversed { first.curve.points[first.curve.degree] } else { first.curve.points[0] };
            let mut area = 0.0;
            let mut magnitude = 0.0;
            // Every edge index is located once through the monotonic ID list.
            for reference in &contour {
                let index = edges.binary_search_by_key(&reference.edge, |edge| edge.id).unwrap();
                let (value, uncertainty_scale) = curve_area(edges[index].curve, local_origin);
                area += if reference.reversed { -value } else { value };
                magnitude += uncertainty_scale;
            }
            if !area.is_finite() || area.abs() <= magnitude * (256.0 * f64::EPSILON) {
                return Err(NetworkFaceError::IndeterminateBoundary);
            }
            let path = contour_path(self, &contour);
            let bounds = path.tight_bounds().ok_or(NetworkFaceError::IndeterminateBoundary)?;
            let boundary = Boundary { contour, area, bounds };
            if area > 0.0 { positive.push(boundary); } else { negative.push(boundary); }
        }
        let mut faces: Vec<_> = positive.iter().map(|boundary| NetworkRegion { contours: vec![boundary.contour.clone()] }).collect();
        let mut containers = crate::SpatialIndex::new(scale / (positive.len().max(1) as f64).sqrt());
        for (index, boundary) in positive.iter().enumerate() { containers.insert(index as u64, boundary.bounds); }
        for hole in negative {
            let reference = hole.contour[0];
            let point = self.edge_curve(reference.edge).unwrap().evaluate(0.5);
            let hole_edges: BTreeSet<_> = hole.contour.iter().map(|r| r.edge).collect();
            let mut parent: Option<usize> = None;
            for index in containers.query(Rect::new(point.x, point.y, 0.0, 0.0)) {
                work.take()?;
                let index = index as usize;
                let outer = &positive[index];
                if !outer.contour.iter().any(|r| hole_edges.contains(&r.edge)) && contour_winding(self, &outer.contour, point) != 0
                    && parent.is_none_or(|previous| outer.area < positive[previous].area)
                { parent = Some(index); }
            }
            if let Some(index) = parent { faces[index].contours.push(hole.contour); }
        }
        for face in &mut faces {
            face.contours.sort();
            if !self.region_is_valid(face) { return Err(NetworkFaceError::IndeterminateBoundary); }
        }
        faces.sort_by_key(region_key);
        Ok(faces)
    }

    /// Returns the elementary face containing a node-local point. The complete
    /// checked embedding is reused for exact curve containment, including holes.
    pub fn face_at(&self, point: Point) -> Result<Option<NetworkRegion>, NetworkFaceError> {
        if !finite(point) { return Ok(None); }
        Ok(self.detect_faces()?.into_iter().find(|face| face.contours.iter()
            .map(|contour| contour_winding(self, contour, point)).sum::<i32>() != 0))
    }

    /// Replaces the selected fill regions with all bounded elementary faces.
    /// Matching regions keep IDs and serialized contour order; vertex/edge IDs,
    /// geometry, fill rule and stroke ownership never change. Preflight errors,
    /// exhausted IDs and repeated invocations leave the entire graph untouched.
    pub fn fill_all_faces(&mut self) -> Result<bool, NetworkFaceError> {
        let faces = self.detect_faces()?;
        let mut existing: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (&id, region) in &self.regions { existing.entry(face_key(region)).or_default().push(id); }
        let mut regions = BTreeMap::new();
        let mut pending = Vec::new();
        for face in faces {
            if let Some(id) = existing.get_mut(&face_key(&face)).and_then(|ids| {
                if ids.is_empty() { None } else { Some(ids.remove(0)) }
            }) {
                regions.insert(id, self.regions[&id].clone());
            } else {
                pending.push(face);
            }
        }
        let (first, next) = self.reserve_ids(pending.len() as u64).ok_or(NetworkFaceError::IdExhausted)?;
        for (index, face) in pending.into_iter().enumerate() { regions.insert(first + index as u64, face); }
        if regions == self.regions { return Ok(false); }
        self.regions = regions;
        self.next_id = next;
        Ok(true)
    }
}

impl Editor {
    /// One scoped, reversible fill-only edit. World transforms and bounds do
    /// not change, and discovery errors preserve selection/history/redo.
    pub fn fill_network_faces(&mut self, id: NodeId) -> Result<bool, NetworkFaceError> {
        if self.document.effective_locked(id) { return Err(NetworkFaceError::NotEditable); }
        let mut node = self.document.nodes.get(&id).cloned().ok_or(NetworkFaceError::NotEditable)?;
        if node.kind != NodeKind::Vector { return Err(NetworkFaceError::NotEditable); }
        let network = node.vector_network.as_mut().ok_or(NetworkFaceError::NotEditable)?;
        let changed = network.fill_all_faces()?;
        let needs_fill = node.fills.is_empty() && !network.regions.is_empty();
        if !changed && !needs_fill { return Ok(false); }
        if needs_fill { node.fills.push(crate::Fill::solid(crate::Color::rgb(217, 217, 217))); }
        // The ordinary property update boundary detaches a conflicting empty
        // fill style when authoring the default fill, within the same history.
        self.update_node(id, "Fill network faces", |existing| *existing = node);
        Ok(true)
    }
}

fn contour_path(network: &VectorNetwork, contour: &[NetworkEdgeRef]) -> VectorPath {
    let mut path = VectorPath { fill_rule: FillRule::NonZero, ..VectorPath::default() };
    if let Some((start, _, _)) = contour.first().and_then(|reference| network.oriented_edge(*reference)) {
        path.segments.push(PathSegment::MoveTo { point: network.vertices[&start].position });
        path.segments.extend(contour.iter().filter_map(|reference| network.oriented_edge(*reference).map(|(_, _, command)| command)));
        path.segments.push(PathSegment::Close);
    }
    path
}

/// Polynomial ray crossings, independent of the document's flattened hit-test
/// path. Derivative roots partition every curve into y-monotone intervals. The
/// half-open y test counts a shared vertex once and cancels local extrema. A
/// bracketed root search then evaluates x on the actual Bézier, including for
/// arbitrarily small artwork and narrow curved annuli.
fn contour_winding(network: &VectorNetwork, contour: &[NetworkEdgeRef], point: Point) -> i32 {
    let mut winding = 0;
    for reference in contour {
        let mut curve = network.edge_curve(reference.edge).unwrap();
        // Translating before multiplication avoids squaring world coordinates;
        // query-relative scaling keeps tiny and translated artwork equivalent.
        curve.points = curve.points.map(|position| position - point);
        let scale = curve.points[..=curve.degree].iter()
            .map(|position| position.x.abs().max(position.y.abs())).fold(0.0, f64::max);
        if scale <= 0.0 || !scale.is_finite() { continue; }
        curve.points = curve.points.map(|position| position * (1.0 / scale));
        let coefficients = curve.power_coefficients();
        let mut derivative = [0.0; 3];
        for index in 1..=curve.degree { derivative[index - 1] = coefficients[index].y * index as f64; }
        let mut breaks = vec![0.0];
        breaks.extend(unit_polynomial_roots(&derivative[..curve.degree]).into_iter().filter(|&t| t > 0.0 && t < 1.0));
        breaks.push(1.0);
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
        for interval in breaks.windows(2) {
            let (mut low, mut high) = (interval[0], interval[1]);
            let (a, b) = (curve.evaluate(low).y, curve.evaluate(high).y);
            let direction = if a <= 0.0 && b > 0.0 { 1 }
                else if b <= 0.0 && a > 0.0 { -1 }
                else { continue; };
            let t = if a == 0.0 { low } else if b == 0.0 { high } else {
                for _ in 0..64 {
                    let middle = low + (high - low) * 0.5;
                    if middle == low || middle == high { break; }
                    let y = curve.evaluate(middle).y;
                    if y == 0.0 { low = middle; high = middle; break; }
                    if (y < 0.0) == (direction > 0) { low = middle; } else { high = middle; }
                }
                low + (high - low) * 0.5
            };
            if curve.evaluate(t).x > 0.0 { winding += if reference.reversed { -direction } else { direction }; }
        }
    }
    winding
}

fn canonicalize_contour(contour: &mut [NetworkEdgeRef]) {
    if let Some((first, _)) = contour.iter().enumerate().min_by_key(|(_, reference)| **reference) { contour.rotate_left(first); }
}

fn face_key(region: &NetworkRegion) -> Vec<Vec<NetworkEdgeRef>> {
    let forward = region_key(region);
    let reverse = region_key(&NetworkRegion { contours: region.contours.iter().map(|contour| {
        contour.iter().rev().map(|reference| NetworkEdgeRef { edge: reference.edge, reversed: !reference.reversed }).collect()
    }).collect() });
    forward.min(reverse)
}

fn curve_area(mut curve: Curve, origin: Point) -> (f64, f64) {
    curve.points = curve.points.map(|point| point - origin);
    let coefficients = curve.power_coefficients();
    let mut area = 0.0;
    let mut magnitude = 0.0;
    for i in 0..=curve.degree {
        for j in 1..=curve.degree {
            let value = (coefficients[i].x * coefficients[j].y - coefficients[i].y * coefficients[j].x)
                * (j as f64 / (i + j) as f64) * 0.5;
            area += value;
            magnitude += value.abs();
        }
    }
    (area, magnitude)
}

fn outgoing_direction(curve: Curve, reversed: bool) -> Option<Point> {
    let origin = if reversed { curve.points[curve.degree] } else { curve.points[0] };
    for index in 1..=curve.degree {
        let point = curve.points[if reversed { curve.degree - index } else { index }] - origin;
        let length = point.x.hypot(point.y);
        if length > 0.0 && length.is_finite() { return Some(point * (1.0 / length)); }
    }
    None
}

// Iterative Tarjan bridge discovery keeps large open networks off the call
// stack. Parallel edges use edge identity, so a two-edge curved lens is a cycle.
fn find_bridges(edges: &[EmbeddedEdge], adjacency: &[Vec<usize>]) -> Vec<bool> {
    let mut bridges = vec![false; edges.len()];
    let mut discovered = vec![usize::MAX; adjacency.len()];
    let mut low = vec![0; adjacency.len()];
    let mut clock = 0;
    for root in 0..adjacency.len() {
        if discovered[root] != usize::MAX { continue; }
        discovered[root] = clock; low[root] = clock; clock += 1;
        let mut stack = vec![(root, usize::MAX, 0)];
        while let Some((vertex, parent_edge, position)) = stack.last_mut() {
            if *position < adjacency[*vertex].len() {
                let edge_index = adjacency[*vertex][*position];
                *position += 1;
                if edge_index == *parent_edge { continue; }
                let edge = edges[edge_index];
                let other = if edge.start == *vertex { edge.end } else { edge.start };
                if discovered[other] == usize::MAX {
                    discovered[other] = clock; low[other] = clock; clock += 1;
                    stack.push((other, edge_index, 0));
                } else {
                    low[*vertex] = low[*vertex].min(discovered[other]);
                }
            } else {
                let (vertex, parent_edge, _) = stack.pop().unwrap();
                if let Some(&(parent, _, _)) = stack.last() {
                    bridges[parent_edge] = low[vertex] > discovered[parent];
                    low[parent] = low[parent].min(low[vertex]);
                }
            }
        }
    }
    bridges
}

fn split_curve(curve: Curve) -> (Curve, Curve) {
    let mut current = curve.points;
    let mut a = [curve.points[0]; 4];
    let mut b = [curve.points[curve.degree]; 4];
    for level in (1..=curve.degree).rev() {
        for i in 0..level { current[i] = interpolate(current[i], current[i + 1], 0.5); }
        a[curve.degree - level + 1] = current[0];
        b[level - 1] = current[level - 1];
    }
    (Curve { points: a, degree: curve.degree }, Curve { points: b, degree: curve.degree })
}

fn monotone(curve: Curve) -> bool {
    [false, true].into_iter().any(|y| {
        let mut differences = [0.0; 3];
        for i in 1..=curve.degree {
            differences[i - 1] = if y { curve.points[i].y - curve.points[i - 1].y } else { curve.points[i].x - curve.points[i - 1].x };
        }
        let differences = &differences[..curve.degree];
        differences.iter().any(|d| *d != 0.0)
            && (differences.iter().all(|d| *d >= 0.0) || differences.iter().all(|d| *d <= 0.0))
    })
}

fn certify_simple_curve(curve: Curve, edge: VectorEdgeId, work: &mut Work) -> Result<(), NetworkFaceError> {
    if monotone(curve) { return Ok(()); }
    let mut pending = vec![(curve, 0u8)];
    let mut pieces = Vec::new();
    while let Some((piece, depth)) = pending.pop() {
        work.take()?;
        if monotone(piece) { pieces.push(piece); continue; }
        if depth == MAX_SUBDIVISION { return Err(NetworkFaceError::DegenerateEdge(edge)); }
        let (a, b) = split_curve(piece);
        pending.push((b, depth + 1)); pending.push((a, depth + 1));
    }
    for (i, &a) in pieces.iter().enumerate() {
        for (j, &b) in pieces.iter().enumerate().skip(i + 1) {
            let mut allowed = Vec::new();
            if j == i + 1 { allowed.push(a.points[a.degree]); }
            if i == 0 && j == pieces.len() - 1 && curve.points[0] == curve.points[curve.degree] { allowed.push(curve.points[0]); }
            certify_disjoint(a, b, &allowed, edge, edge, work)?;
        }
    }
    Ok(())
}

fn certify_embedding(edges: &[EmbeddedEdge], work: &mut Work) -> Result<(), NetworkFaceError> {
    let mut sweep: Vec<_> = (0..edges.len()).collect();
    sweep.sort_by(|&a, &b| edges[a].bounds.x.total_cmp(&edges[b].bounds.x).then(a.cmp(&b)));
    for (i, &a) in sweep.iter().enumerate() {
        let a = edges[a];
        for &b in sweep.iter().skip(i + 1) {
            let b = edges[b];
            if b.bounds.x > a.bounds.x + a.bounds.width { break; }
            work.take()?;
            if a.bounds.y + a.bounds.height < b.bounds.y || b.bounds.y + b.bounds.height < a.bounds.y { continue; }
            let mut allowed = Vec::new();
            for (vertex, point) in [(a.start, a.curve.points[0]), (a.end, a.curve.points[a.curve.degree])] {
                if (vertex == b.start || vertex == b.end) && !allowed.contains(&point) { allowed.push(point); }
            }
            certify_disjoint(a.curve, b.curve, &allowed, a.id, b.id, work)?;
        }
    }
    Ok(())
}

fn certify_disjoint(a: Curve, b: Curve, allowed: &[Point], a_id: VectorEdgeId, b_id: VectorEdgeId, work: &mut Work) -> Result<(), NetworkFaceError> {
    let mut pending = vec![(a, b, 0u8)];
    while let Some((a, b, depth)) = pending.pop() {
        work.take()?;
        if separated_hulls(a, b, allowed) { continue; }
        if (a.degree == 1 && b.degree == 1) || depth == MAX_SUBDIVISION {
            return Err(NetworkFaceError::UnsplitIntersection(a_id, b_id));
        }
        let a_bounds = a.hull_bounds();
        let b_bounds = b.hull_bounds();
        if (a_bounds.width + a_bounds.height >= b_bounds.width + b_bounds.height && a.degree != 1) || b.degree == 1 {
            let (first, second) = split_curve(a);
            pending.push((second, b, depth + 1)); pending.push((first, b, depth + 1));
        } else {
            let (first, second) = split_curve(b);
            pending.push((a, second, depth + 1)); pending.push((a, first, depth + 1));
        }
    }
    Ok(())
}

fn separated_hulls(a: Curve, b: Curve, allowed: &[Point]) -> bool {
    if separating_axis(a, b, Point::new(1.0, 0.0), allowed)
        || separating_axis(a, b, Point::new(0.0, 1.0), allowed)
    { return true; }
    for curve in [a, b] {
        for i in 0..=curve.degree {
            for j in i + 1..=curve.degree {
                let direction = curve.points[j] - curve.points[i];
                if direction == Point::default() { continue; }
                if separating_axis(a, b, Point::new(-direction.y, direction.x), allowed)
                    || separating_axis(a, b, direction, allowed)
                { return true; }
            }
        }
    }
    false
}

fn separating_axis(a: Curve, b: Curve, axis: Point, allowed: &[Point]) -> bool {
    let project = |point: Point| point.x * axis.x + point.y * axis.y;
    let range = |curve: Curve| {
        curve.points[..=curve.degree].iter().map(|&point| project(point))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), value| (lo.min(value), hi.max(value)))
    };
    let (a_min, a_max) = range(a);
    let (b_min, b_max) = range(b);
    if a_max < b_min || b_max < a_min { return true; }
    let plane = if a_max == b_min { a_max } else if b_max == a_min { b_max } else { return false; };
    // A nonconstant Bézier projection has a strict sign in its interior when
    // all controls lie on one side. Its endpoints on the separator must be
    // explicitly allowed shared vertices; coincident whole curves are declined.
    let a_off = a_min != a_max;
    let b_off = b_min != b_max;
    if !a_off && !b_off { return false; }
    [a, b].into_iter().zip([a_off, b_off]).all(|(curve, off)| {
        !off || [curve.points[0], curve.points[curve.degree]].into_iter()
            .all(|point| project(point) != plane || allowed.contains(&point))
    })
}

#[cfg(test)]
#[path = "vector_faces_tests.rs"]
mod tests;
