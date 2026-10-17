use super::diagnostic_text_renderer::ShapedText;
use super::DiagnosticTextRenderer;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Brushes, Colors, IBrush};
use crate::platform::IDrawingContextImpl;
use crate::utilities::ByteSizeHelper;
use crate::{Matrix, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const FRAME_DIGITS: i32 = 8;
const FPS_DIGITS: i32 = 3;
const VISUAL_DIGITS: i32 = 5;

/// An FPS counter helper that can draw itself on the render thread.
/// The labels are shaped once and the changing numbers are drawn in a fixed digit grid,
/// so a frame neither formats nor measures any text.
///
/// The C# class owns a stopwatch started at construction. Here the caller
/// supplies the time instead: [`new`](Self::new) and
/// [`render_fps`](Self::render_fps) take the current reading of a monotonic
/// clock (any epoch), and the counter works with the time elapsed between
/// the two.
pub struct FpsCounter {
    start: Duration,
    text_renderer: Rc<DiagnosticTextRenderer>,
    frame_label: Rc<ShapedText>,
    fps_label: Rc<ShapedText>,
    visited_label: Rc<ShapedText>,
    rendered_label: Rc<ShapedText>,
    digits_width: f64,
    line_height: f64,

    /// The memory text, `None` while no memory is reported.
    memory_text: RefCell<Option<ShapedText>>,
    memory: Cell<Option<(u64, u64)>>,
    text_width: Cell<f64>,

    frames_this_second: Cell<i32>,
    total_frames: Cell<i32>,
    fps: Cell<i32>,
    last_fps_update: Cell<Duration>,
}

impl FpsCounter {
    /// The fixed labels of the counter. The compositor shapes them with the text renderer on
    /// the UI thread (`ServerCompositor::diagnostic_text_renderer`), so that a counter created
    /// by the first frame of the render thread finds them shaped: shaping creates glyph runs,
    /// whose platform implementation comes from the services of the UI thread (the render
    /// thread of the browser has none). Upstream shapes them in the constructor.
    pub const LABELS: [&'static str; 4] = ["Frame #", " FPS: ", " V:", " R:"];

    /// `now` is the current reading of the monotonic clock that is also
    /// passed to [`render_fps`](Self::render_fps).
    pub fn new(text_renderer: Rc<DiagnosticTextRenderer>, now: Duration) -> Self {
        let counter = Self {
            start: now,
            frame_label: text_renderer.shape_label(Self::LABELS[0]),
            fps_label: text_renderer.shape_label(Self::LABELS[1]),
            visited_label: text_renderer.shape_label(Self::LABELS[2]),
            rendered_label: text_renderer.shape_label(Self::LABELS[3]),
            digits_width: (FRAME_DIGITS + FPS_DIGITS + 2 * VISUAL_DIGITS) as f64 * text_renderer.digit_cell_width(),
            line_height: text_renderer.get_max_height(),
            text_renderer,
            memory_text: RefCell::new(None),
            memory: Cell::new(None),
            text_width: Cell::new(0.0),
            frames_this_second: Cell::new(0),
            total_frames: Cell::new(0),
            fps: Cell::new(0),
            last_fps_update: Cell::new(Duration::ZERO),
        };
        counter.update_text_width();
        counter
    }

    pub fn fps_tick(&self) {
        self.frames_this_second.set(self.frames_this_second.get().wrapping_add(1));
    }

    /// Counts a frame and draws the counter at the origin of the current
    /// transform of `context`. Returns the rectangle that was covered.
    ///
    /// `memory` is the managed and the native size of the batch pools of
    /// the compositor in bytes (`M:` and `N:`), or `None` to draw no memory
    /// text: the compositor of the port has no such pools (see the member
    /// waivers of `ServerCompositor`).
    // Deviation (DEVIATIONS.md, Server composition visuals): upstream takes the two sizes and always
    // draws the memory text.
    #[allow(clippy::too_many_arguments)]
    pub fn render_fps(
        &self,
        context: &mut dyn IDrawingContextImpl,
        memory: Option<(u64, u64)>,
        visited_visuals: i32,
        rendered_visuals: i32,
        has_layer: bool,
        old_rect: Option<Rect>,
        now: Duration,
    ) -> Rect {
        let now = now.saturating_sub(self.start);
        let elapsed = now.saturating_sub(self.last_fps_update.get());

        self.frames_this_second.set(self.frames_this_second.get().wrapping_add(1));
        self.total_frames.set(self.total_frames.get().wrapping_add(1));

        if elapsed.as_secs_f64() > 1.0 {
            self.fps.set((self.frames_this_second.get() as f64 / elapsed.as_secs_f64()) as i32);
            self.frames_this_second.set(0);
            self.last_fps_update.set(now);
        }

        if memory != self.memory.get() {
            self.memory.set(memory);
            let memory_text = self.create_memory_text();
            if let Some(old) = self.memory_text.replace(memory_text) {
                old.dispose();
            }
            self.update_text_width();
        }

        let mut rect = Rect::new(0.0, 0.0, self.text_width.get() + 3.0, self.line_height + 3.0);
        if let (true, Some(old_rect)) = (has_layer, old_rect) {
            rect = rect.union(old_rect);
        }

        let layer_brush = ImmutableSolidColorBrush::with_opacity(Colors::BLACK, 0.5);
        let black = Brushes::black();
        let background: &dyn IBrush = if has_layer { &layer_brush } else { &*black };
        context.draw_rectangle(Some(background), None, rect.into(), &BoxShadows::default());

        let mut x = 0.0;
        self.draw_label(context, &*self.frame_label, &mut x);
        self.draw_number(context, self.total_frames.get(), FRAME_DIGITS, &mut x);
        self.draw_label(context, &*self.fps_label, &mut x);
        self.draw_number(context, self.fps.get(), FPS_DIGITS, &mut x);
        if let Some(memory_text) = &*self.memory_text.borrow() {
            self.draw_label(context, memory_text, &mut x);
        }
        self.draw_label(context, &*self.visited_label, &mut x);
        self.draw_number(context, visited_visuals, VISUAL_DIGITS, &mut x);
        self.draw_label(context, &*self.rendered_label, &mut x);
        self.draw_number(context, rendered_visuals, VISUAL_DIGITS, &mut x);

        rect
    }

    fn draw_label(&self, context: &mut dyn IDrawingContextImpl, text: &ShapedText, x: &mut f64) {
        let old_transform = context.transform();
        context.set_transform(Matrix::create_translation(*x, 0.0) * old_transform);
        self.text_renderer.draw_shaped_text(context, text, &*Brushes::white());
        context.set_transform(old_transform);

        *x += text.width();
    }

    fn draw_number(&self, context: &mut dyn IDrawingContextImpl, value: i32, digit_count: i32, x: &mut f64) {
        let old_transform = context.transform();
        context.set_transform(Matrix::create_translation(*x, 0.0) * old_transform);
        self.text_renderer.draw_digits(context, value, digit_count, &*Brushes::white());
        context.set_transform(old_transform);

        *x += digit_count as f64 * self.text_renderer.digit_cell_width();
    }

    fn create_memory_text(&self) -> Option<ShapedText> {
        let (managed_bytes, native_bytes) = self.memory.get()?;
        Some(self.text_renderer.shape(&format!(
            " M:{} / N:{}",
            ByteSizeHelper::to_string(managed_bytes, false),
            ByteSizeHelper::to_string(native_bytes, false)
        )))
    }

    fn update_text_width(&self) {
        let memory_width = self.memory_text.borrow().as_ref().map_or(0.0, ShapedText::width);
        self.text_width.set(
            self.frame_label.width()
                + self.fps_label.width()
                + self.visited_label.width()
                + self.rendered_label.width()
                + memory_width
                + self.digits_width,
        );
    }

    pub fn reset(&self) {
        self.frames_this_second.set(0);
        self.total_frames.set(0);
        self.fps.set(0);
    }
}

#[cfg(test)]
mod tests {
    use super::super::diagnostic_text_renderer::tests::create_renderer;
    use super::super::dirty_rects::test_mocks::MockDrawingContext;
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    /// The text drawn by the last `render_fps`, rebuilt from the glyph log:
    /// single glyphs by their character, shaped runs by their text.
    fn drawn_text(ctx: &MockDrawingContext) -> String {
        ctx.log
            .iter()
            .filter_map(|l| {
                if let Some(glyph) = l.strip_prefix("glyph ") {
                    let code = glyph.split(' ').next().unwrap().parse::<u32>().unwrap();
                    Some(char::from_u32(code).unwrap().to_string())
                } else {
                    let text = l.strip_prefix("text ")?;
                    let at = text.rfind(" at ").unwrap();
                    Some(text[..at].rsplit_once(' ').unwrap().0.to_string())
                }
            })
            .collect()
    }

    fn render(counter: &FpsCounter, visited: i32, rendered: i32, now: Duration) -> String {
        let mut ctx = MockDrawingContext::default();
        counter.render_fps(&mut ctx, None, visited, rendered, false, None, now);
        drawn_text(&ctx)
    }

    #[test]
    fn counts_frames_and_updates_fps_once_more_than_a_second_elapsed() {
        // The clock does not start at zero: only the elapsed time matters.
        let start = ms(5000);
        let counter = FpsCounter::new(Rc::new(create_renderer()), start);

        assert_eq!("Frame #00000001 FPS: 000 V:00003 R:00002", render(&counter, 3, 2, start + ms(100)));
        assert_eq!("Frame #00000002 FPS: 000 V:00000 R:00000", render(&counter, 0, 0, start + ms(500)));
        // Exactly one second is not enough.
        assert_eq!("Frame #00000003 FPS: 000 V:00000 R:00000", render(&counter, 0, 0, start + ms(1000)));
        // 4 frames in 2 seconds.
        assert_eq!("Frame #00000004 FPS: 002 V:00000 R:00000", render(&counter, 0, 0, start + ms(2000)));
        // The window restarted at 2s; fps is kept until the next update.
        assert_eq!("Frame #00000005 FPS: 002 V:00000 R:00000", render(&counter, 0, 0, start + ms(2500)));
        // 2 frames in 1.25 seconds: 1.6 truncates to 1.
        assert_eq!("Frame #00000006 FPS: 001 V:00000 R:00000", render(&counter, 0, 0, start + ms(3250)));
    }

    #[test]
    fn fps_tick_counts_extra_frames() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        for _ in 0..119 {
            counter.fps_tick();
        }
        assert_eq!("Frame #00000001 FPS: 060 V:00000 R:00000", render(&counter, 0, 0, ms(2000)));
    }

    #[test]
    fn reset_clears_counters_but_not_the_update_time() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        render(&counter, 0, 0, ms(1500));
        render(&counter, 0, 0, ms(1600));
        counter.reset();
        assert_eq!("Frame #00000001 FPS: 000 V:00000 R:00000", render(&counter, 0, 0, ms(1700)));
        // 2 frames since the reset, 1.5s since the last update at 1.5s.
        assert_eq!("Frame #00000002 FPS: 001 V:00000 R:00000", render(&counter, 0, 0, ms(3000)));
    }

    #[test]
    fn numbers_are_clamped_to_their_digits() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        assert_eq!("Frame #00000001 FPS: 000 V:99999 R:00000", render(&counter, 123456, -1, ms(10)));
    }

    #[test]
    fn the_memory_text_follows_the_frames_per_second() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        let mut ctx = MockDrawingContext::default();
        counter.render_fps(&mut ctx, Some((1024, 2048)), 1, 2, false, None, ms(10));
        let expected = format!(
            "Frame #00000001 FPS: 000 M:{} / N:{} V:00001 R:00002",
            ByteSizeHelper::to_string(1024, false),
            ByteSizeHelper::to_string(2048, false)
        );
        assert_eq!(expected, drawn_text(&ctx));
    }

    #[test]
    fn draws_background_then_text_and_returns_the_rect() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        let mut ctx = MockDrawingContext::default();
        // "Frame #" (7), " FPS: " (6), " V:" (3), " R:" (3) and 21 digit cells, all 6 wide:
        // 40 glyphs; the line is 14 high.
        let rect = counter.render_fps(&mut ctx, None, 0, 0, false, Some(Rect::new(0.0, 0.0, 500.0, 500.0)), ms(10));
        assert_eq!(Rect::new(0.0, 0.0, 243.0, 17.0), rect);
        assert_eq!("rect #ff000000@1 0, 0, 243, 17", ctx.log[0]);
        // The background, four labels and 21 digits.
        assert_eq!(26, ctx.log.len());
        assert_eq!("text Frame # #ffffffff@1 at 0,0", ctx.log[1]);
        assert_eq!("glyph 48 #ffffffff@1 at 42,0", ctx.log[2]);
    }

    #[test]
    fn with_a_layer_the_old_rect_is_covered_with_a_translucent_background() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        let mut ctx = MockDrawingContext::default();
        let rect = counter.render_fps(&mut ctx, None, 0, 0, true, Some(Rect::new(0.0, 0.0, 300.0, 10.0)), ms(10));
        assert_eq!(Rect::new(0.0, 0.0, 300.0, 17.0), rect);
        assert_eq!("rect #ff000000@0.5 0, 0, 300, 17", ctx.log[0]);

        let mut ctx = MockDrawingContext::default();
        let rect = counter.render_fps(&mut ctx, None, 0, 0, true, None, ms(20));
        assert_eq!(Rect::new(0.0, 0.0, 243.0, 17.0), rect);
    }
}
