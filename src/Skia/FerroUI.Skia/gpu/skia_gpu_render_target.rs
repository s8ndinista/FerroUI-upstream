use super::{ISkiaGpu, ISkiaGpuRenderTarget};
use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::skia_platform::SkiaPlatform;
use crate::text_rendering_settings::TextRenderingSource;
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties,
    RenderTargetError, RenderTargetProperties, RenderTargetSceneInfo,
};
use std::rc::Rc;

/// Adapts a GPU render target to the platform render target contract.
pub struct SkiaGpuRenderTarget {
    skia_gpu: Rc<dyn ISkiaGpu>,
    render_target: Rc<dyn ISkiaGpuRenderTarget>,
    text_rendering: TextRenderingSource,
}

impl SkiaGpuRenderTarget {
    /// Creates a render target that renders with `skia_gpu` to
    /// `render_target`, with the default text rendering of the backend.
    pub fn new(skia_gpu: Rc<dyn ISkiaGpu>, render_target: Rc<dyn ISkiaGpuRenderTarget>) -> Self {
        Self::with_text_rendering(skia_gpu, render_target, TextRenderingSource::default())
    }

    /// Creates a render target that renders with `skia_gpu` to
    /// `render_target` and reads the text rendering parameters of the
    /// platform from `text_rendering` every frame.
    pub fn with_text_rendering(
        skia_gpu: Rc<dyn ISkiaGpu>,
        render_target: Rc<dyn ISkiaGpuRenderTarget>,
        text_rendering: TextRenderingSource,
    ) -> Self {
        Self { skia_gpu, render_target, text_rendering }
    }
}

impl IRenderTarget for SkiaGpuRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        match self.try_create_drawing_context(scene_info) {
            Ok(created) => created,
            Err(error) => panic!("{error}"),
        }
    }

    fn try_create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<(Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties), RenderTargetError> {
        let text_rendering = self.text_rendering.settings(scene_info.scaling, scene_info.transparency_level);
        let session = self.render_target.try_begin_rendering_session(scene_info, &text_rendering.surface_props())?;

        let nfo = CreateInfo {
            gr_context: Some(session.gr_context()),
            surface: Some(session.sk_surface()),
            dpi: SkiaPlatform::default_dpi() * session.scale_factor(),
            scale_drawing_to_dpi: false,
            gpu: Some(self.skia_gpu.clone()),
            current_session: Some(session.clone()),
            text_rendering,
            ..CreateInfo::default()
        };

        let context = DrawingContextImpl::new(nfo, vec![Box::new(move || session.dispose())]);

        Ok((Box::new(context), RenderTargetDrawingContextProperties::default()))
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        self.render_target.state()
    }

    fn dispose(&self) {
        self.render_target.dispose();
    }
}
