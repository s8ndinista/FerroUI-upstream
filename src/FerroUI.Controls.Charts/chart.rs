use crate::chart_collection::CollectionOwner;
use crate::rendering::{ChartLayout, ChartScene};
use crate::{ChartAutomationPeer, ChartAxisCollection, ChartDataCollection, ChartSeriesCollection, Samples};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::DrawingContext;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::ControlImpl;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Displays cartesian line, area and bar series with shared, observable data.
/// Drawing goes through the framework's retained drawing context on every backend.
#[repr(C)]
pub struct Chart {
    base: TemplatedControl,
    owner: Rc<CollectionOwner>,
    series: ChartSeriesCollection,
    axes: ChartAxisCollection,
    data: ChartDataCollection,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    resource_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    scene: RefCell<Option<Rc<ChartScene>>>,
    revision: Cell<u64>,
    scene_build_count: Cell<u64>,
}

ferro_class!(Chart: TemplatedControl);
ferro_class_info!(Chart {
    new: Chart::new,
    markup: {
        content: Series,
        properties: [
            Series: ChartSeriesCollection { get: Chart::series },
            Axes: ChartAxisCollection { get: Chart::axes },
            Data: ChartDataCollection { get: Chart::data },
        ],
    },
});
ferro_impl_classes!(Chart: StyledElementImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl);

ferro_properties! { impl Chart {
    pub fn show_legend_property() -> StyledProperty<bool> {
        FerroProperty::register::<Chart, _>("ShowLegend", false)
    }
    pub fn legend_title_property() -> StyledProperty<String> {
        FerroProperty::register::<Chart, _>("LegendTitle", String::new())
    }
} }

impl FerroObjectImpl for Chart {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        *this.owner.changed.borrow_mut() = Some(Rc::new(move || {
            if let Some(chart) = weak.upgrade() {
                chart.refresh_subscriptions();
                chart.invalidate_scene();
            }
        }));
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(chart) = weak.upgrade() { chart.invalidate_scene(); }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(chart) = weak.upgrade() { chart.invalidate_scene(); }
        });
        this.resource_subscriptions.borrow_mut().extend([resources, theme]);
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        this.invalidate_scene();
    }
}

impl LayoutableImpl for Chart {
    fn measure_override(_this: &Self, available: Size) -> Size {
        Size::new(
            if available.width.is_finite() { available.width.max(0.0) } else { 160.0 },
            if available.height.is_finite() { available.height.max(0.0) } else { 96.0 },
        )
    }
    fn arrange_override(_this: &Self, size: Size) -> Size { size }
}

impl VisualImpl for Chart {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bounds = this.bounds();
        this.scene_for(Size::new(bounds.width, bounds.height)).draw(context);
    }
}

impl ControlImpl for Chart {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ChartAutomationPeer::new(this).upcast()
    }
}

impl Chart {
    pub fn construct() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let owner = CollectionOwner::new(NEXT_ID.fetch_add(1, Ordering::Relaxed));
        Self {
            base: TemplatedControl::construct(),
            series: ChartSeriesCollection::new(owner.clone()),
            axes: ChartAxisCollection::new(owner.clone()),
            data: ChartDataCollection::new(owner.clone()),
            owner,
            subscriptions: RefCell::new(Vec::new()),
            resource_subscriptions: RefCell::new(Vec::new()),
            scene: RefCell::new(None),
            revision: Cell::new(0),
            scene_build_count: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> { instantiate(Self::construct()) }
    pub fn show_legend(&self) -> bool { self.get_value(Self::show_legend_property()) }
    pub fn set_show_legend(&self, value: bool) { self.set_value(Self::show_legend_property(), value); }
    pub fn legend_title(&self) -> String { self.get_value(Self::legend_title_property()) }
    pub fn set_legend_title(&self, value: impl Into<String>) {
        self.set_value(Self::legend_title_property(), value.into());
    }
    pub fn series(&self) -> ChartSeriesCollection { self.series.clone() }
    pub fn axes(&self) -> ChartAxisCollection { self.axes.clone() }
    pub fn data(&self) -> ChartDataCollection { self.data.clone() }
    /// Increases on a data, collection, style or layout-related property change.
    pub fn revision(&self) -> u64 { self.revision.get() }
    /// Number of retained scene rebuilds. Unchanged paints reuse the same scene.
    pub fn scene_build_count(&self) -> u64 { self.scene_build_count.get() }
    /// Returns the calculated plot and point positions without drawing. A normal
    /// framework text service is required, as for measuring any text control.
    pub fn layout_snapshot(&self, size: Size) -> ChartLayout { self.scene_for(size).layout.clone() }

    pub(crate) fn invalidate_scene(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
        self.scene.borrow_mut().take();
        self.invalidate_visual();
    }

    fn scene_for(&self, size: Size) -> Rc<ChartScene> {
        let scene = self.scene.borrow().clone();
        if let Some(scene) = scene {
            if scene.layout.size == size { return scene; }
        }
        let scene = Rc::new(ChartScene::build(self, size));
        self.scene_build_count.set(self.scene_build_count.get().wrapping_add(1));
        *self.scene.borrow_mut() = Some(scene.clone());
        scene
    }

    fn refresh_subscriptions(&self) {
        // Dispose outside the borrow: subscriptions are allowed to call user code.
        let previous = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in previous { subscription.dispose(); }
        let mut subscriptions = Vec::new();
        let mut samples = self.data.snapshot().as_ref().clone();
        for axis in self.axes.snapshot().iter() {
            let weak = self.to_ref().downgrade();
            subscriptions.push(axis.property_changed(move |_| {
                if let Some(chart) = weak.upgrade() { chart.invalidate_scene(); }
            }));
        }
        for series in self.series.snapshot().iter() {
            let weak = self.to_ref().downgrade();
            subscriptions.push(series.property_changed(move |_| {
                if let Some(chart) = weak.upgrade() {
                    chart.refresh_subscriptions();
                    chart.invalidate_scene();
                }
            }));
            for sample in [series.x_values(), series.y_values()].into_iter().flatten() {
                if !samples.contains(&sample) { samples.push(sample); }
            }
            let labels = series.data_label_overrides();
            let weak = self.to_ref().downgrade();
            let token = labels.add_collection_changed(Rc::new(move |_| {
                if let Some(chart) = weak.upgrade() { chart.invalidate_scene(); }
            }));
            subscriptions.push(Disposable::create(move || { labels.remove_collection_changed(token); }));
            let markers = series.data_marker_overrides();
            let weak = self.to_ref().downgrade();
            let token = markers.add_collection_changed(Rc::new(move |_| {
                if let Some(chart) = weak.upgrade() { chart.invalidate_scene(); }
            }));
            subscriptions.push(Disposable::create(move || { markers.remove_collection_changed(token); }));
        }
        for sample in samples { subscriptions.push(self.observe_samples(sample)); }
        *self.subscriptions.borrow_mut() = subscriptions;
    }

    fn observe_samples(&self, samples: Ref<Samples>) -> Rc<dyn IDisposable> {
        let weak = self.to_ref().downgrade();
        let token = samples.add_changed(Rc::new(move || {
            if let Some(chart) = weak.upgrade() { chart.invalidate_scene(); }
        }));
        Disposable::create(move || { samples.remove_changed(token); })
    }
}

impl Drop for Chart {
    fn drop(&mut self) {
        self.owner.changed.borrow_mut().take();
        self.owner.id.set(None);
        self.series.release_ownership();
        self.axes.release_ownership();
        for subscription in self.subscriptions.get_mut().drain(..) { subscription.dispose(); }
        for subscription in self.resource_subscriptions.get_mut().drain(..) { subscription.dispose(); }
    }
}
