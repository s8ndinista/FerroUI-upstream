//! Smoothed geometry must agree across retained artwork, child clips and undo.
use super::*;
use ferrodesign_core::Fill;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn render(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    let target = RenderTargetBitmap::new(PixelSize::new(128, 128));
    let stats = {
        let mut context = target.create_drawing_context();
        painter.render_editor(&mut context, editor, &editor.document.pages[0].children,
            Rect::new(0.0, 0.0, 128.0, 128.0))
    };
    let mut pixels = vec![0; 128 * 128 * 4];
    target.copy_pixels(PixelRect::new(0, 0, 128, 128), &mut pixels, 128 * 4);
    assert_eq!(stats.invalid_nodes, 0);
    assert_eq!(stats.unsupported_effects, 0);
    (pixels, stats)
}

fn fixture(kind: NodeKind) -> (Editor, NodeId) {
    let mut doc = Document::new("Shared corner geometry");
    let mut node = Node::new(kind, "Shape", 12.0, 12.0, 104.0, 104.0);
    node.corner_radius = 24.0;
    node.corner_radii = Some([24.0, 32.0, 18.0, 12.0]);
    node.corner_smoothing = 0.6;
    node.fills = vec![Fill::solid(Color::rgb(25, 90, 210))];
    let id = doc.insert_node(doc.pages[0].id, node).unwrap();
    (Editor::new(doc), id)
}

#[test]
fn native_smoothed_shape_equals_its_public_cubic_outline() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (editor, id) = fixture(NodeKind::Rectangle);
    let pixels = render(&mut ScenePainter::default(), &editor).0;
    let mut doc = editor.document.clone();
    let node = doc.nodes.get_mut(&id).unwrap();
    node.path = ferrodesign_core::corner_path(node);
    node.kind = NodeKind::Vector;
    node.corner_smoothing = 0.0;
    node.corner_radius = 0.0;
    node.corner_radii = None;
    let vector = render(&mut ScenePainter::default(), &Editor::new(doc)).0;
    assert_eq!(pixels, vector);
    assert!(pixels.chunks_exact(4).any(|p| p[3] == 255));
    scope.dispose();
}

#[test]
fn smoothed_child_clip_uses_the_same_outline_as_the_parent_fill() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (editor, id) = fixture(NodeKind::Frame);
    let expected = render(&mut ScenePainter::default(), &editor).0;
    let mut doc = editor.document.clone();
    doc.nodes.get_mut(&id).unwrap().fills.clear();
    let child = Node::new(NodeKind::Rectangle, "Oversized child", -20.0, -20.0, 144.0, 144.0)
        .with_fill(Color::rgb(25, 90, 210));
    doc.insert_node(id, child).unwrap();
    let clipped = render(&mut ScenePainter::default(), &Editor::new(doc)).0;
    assert!(expected.iter().zip(&clipped).all(|(a, b)| a.abs_diff(*b) <= 2),
        "A child clip must follow the actual smooth frame outline");
    scope.dispose();
}

#[test]
fn smoothing_invalidates_one_retained_visual_and_history_restores_pixels() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(NodeKind::Rectangle);
    let mut painter = ScenePainter::default();
    let before = render(&mut painter, &editor).0;
    let repeated = render(&mut painter, &editor);
    assert_eq!(before, repeated.0);
    assert_eq!(repeated.1.rebuilt_visuals, 0);
    assert_eq!(editor.set_corner_smoothing(&[id], 0.0).unwrap(), 1);
    let after = render(&mut painter, &editor);
    assert_eq!(after.1.rebuilt_visuals, 1);
    assert_ne!(before, after.0);
    assert!(editor.undo());
    assert_eq!(render(&mut painter, &editor).0, before);
    assert!(editor.redo());
    assert_eq!(render(&mut painter, &editor).0, after.0);
    scope.dispose();
}
