//! The native WinUI Repeater port through the real XAML compiler and runtime.
//! These tests cross the markup/type-system boundary: layouts and factories
//! remain native FerroUI objects, and templates create ordinary controls.

use ferroui_base::layout::Orientation;
use ferroui_base::{Rect, Ref, Size, Thickness};
use ferroui_controls::{
    Border, Control, FlowLayout, FlowLayoutLineAlignment, ItemsRepeater, ItemsRepeaterScrollHost,
    ItemsSource, LayoutPanel, LinedFlowLayout, LinedFlowLayoutItemsJustification,
    LinedFlowLayoutItemsStretch, RecyclePool, RecyclingElementFactory, StackLayout,
    UniformGridLayout, UniformGridLayoutItemsJustification, UniformGridLayoutItemsStretch,
    UserControl,
};

use crate::support::app::xaml_test_base;
use crate::support::loader::load_as;

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

fn realize(repeater: &Ref<ItemsRepeater>, count: i32) {
    repeater.set_items_source(Some(ItemsSource::from_values(0..count)));
    repeater.measure(Size::new(320.0, 240.0));
    repeater.arrange(Rect::new(0.0, 0.0, 320.0, 240.0));
}

#[test]
fn stack_layout_and_implicit_data_template_are_native_objects() {
    let _scope = xaml_test_base();
    let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
        "<ItemsRepeater {NS} HorizontalCacheLength='1.5' VerticalCacheLength='3'>
           <ItemsRepeater.Layout><StackLayout Orientation='Horizontal' Spacing='7' IsVirtualizationEnabled='False'/></ItemsRepeater.Layout>
           <DataTemplate><Border Width='20' Height='16'/></DataTemplate>
         </ItemsRepeater>"
    ));
    let layout = repeater.layout().unwrap().cast::<StackLayout>().unwrap();
    assert_eq!(layout.orientation(), Orientation::Horizontal);
    assert_eq!(layout.spacing(), 7.0);
    assert!(!layout.is_virtualization_enabled());
    assert_eq!(repeater.horizontal_cache_length(), 1.5);
    assert_eq!(repeater.vertical_cache_length(), 3.0);
    realize(&repeater, 4);
    let first = repeater.try_get_element(0).unwrap();
    let second = repeater.try_get_element(1).unwrap();
    assert!(first.is::<Border>());
    assert_eq!(first.bounds().width, 20.0);
    assert_eq!(second.bounds().x, 27.0);
    assert_eq!(repeater.get_element_index(&second), 1);
}

#[test]
fn uniform_grid_layout_properties_and_enumerations_load() {
    let _scope = xaml_test_base();
    let layout = load_as::<Ref<UniformGridLayout>>(&format!(
        "<UniformGridLayout {NS} Orientation='Vertical' MinItemWidth='48' MinItemHeight='24'
          MinRowSpacing='3' MinColumnSpacing='5' MaximumRowsOrColumns='4'
          ItemsJustification='SpaceEvenly' ItemsStretch='Uniform'/>"
    ));
    assert_eq!(layout.orientation(), Orientation::Vertical);
    assert_eq!(layout.min_item_width(), 48.0);
    assert_eq!(layout.min_item_height(), 24.0);
    assert_eq!(layout.min_row_spacing(), 3.0);
    assert_eq!(layout.min_column_spacing(), 5.0);
    assert_eq!(layout.maximum_rows_or_columns(), 4);
    assert_eq!(
        layout.items_justification(),
        UniformGridLayoutItemsJustification::SpaceEvenly
    );
    assert_eq!(
        layout.items_stretch(),
        UniformGridLayoutItemsStretch::Uniform
    );
}

#[test]
fn flow_layout_properties_and_enumerations_load() {
    let _scope = xaml_test_base();
    let layout = load_as::<Ref<FlowLayout>>(&format!(
        "<FlowLayout {NS} Orientation='Horizontal' MinRowSpacing='6' MinColumnSpacing='9' LineAlignment='SpaceBetween'/>"
    ));
    assert_eq!(layout.orientation(), Orientation::Horizontal);
    assert_eq!(layout.min_row_spacing(), 6.0);
    assert_eq!(layout.min_column_spacing(), 9.0);
    assert_eq!(
        layout.line_alignment(),
        FlowLayoutLineAlignment::SpaceBetween
    );
}

#[test]
fn lined_flow_layout_properties_and_enumerations_load() {
    let _scope = xaml_test_base();
    let layout = load_as::<Ref<LinedFlowLayout>>(&format!(
        "<LinedFlowLayout {NS} LineHeight='72' LineSpacing='8' MinItemSpacing='4'
          ItemsJustification='Center' ItemsStretch='Fill'/>"
    ));
    assert_eq!(layout.line_height(), 72.0);
    assert_eq!(layout.line_spacing(), 8.0);
    assert_eq!(layout.min_item_spacing(), 4.0);
    assert_eq!(
        layout.items_justification(),
        LinedFlowLayoutItemsJustification::Center
    );
    assert_eq!(layout.items_stretch(), LinedFlowLayoutItemsStretch::Fill);
}

#[test]
fn recycling_factory_populates_its_nullable_native_dictionary() {
    let _scope = xaml_test_base();
    let factory = load_as::<Ref<RecyclingElementFactory>>(&format!(
        "<RecyclingElementFactory {NS}>
           <DataTemplate x:Key='narrow'><Border Width='24' Height='16'/></DataTemplate>
           <DataTemplate x:Key='wide'><Border Width='48' Height='16'/></DataTemplate>
         </RecyclingElementFactory>"
    ));
    assert_eq!(factory.templates().count(), 2);
    let narrow = factory
        .templates()
        .try_get_value(&"narrow".to_owned())
        .unwrap();
    let wide = factory
        .templates()
        .try_get_value(&"wide".to_owned())
        .unwrap();
    assert_eq!(narrow.build(&None).unwrap().width(), 24.0);
    assert_eq!(wide.build(&None).unwrap().width(), 48.0);
}

#[test]
fn explicit_factory_templates_property_populates_existing_dictionary() {
    let _scope = xaml_test_base();
    let factory = load_as::<Ref<RecyclingElementFactory>>(&format!(
        "<RecyclingElementFactory {NS}>
           <RecyclingElementFactory.Templates>
             <DataTemplate x:Key='item'><Border Width='24' Height='16'/></DataTemplate>
           </RecyclingElementFactory.Templates>
         </RecyclingElementFactory>"
    ));
    assert_eq!(factory.templates().count(), 1);
}

#[test]
fn resource_factory_and_recycle_pool_are_shared_native_instances() {
    let _scope = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Resources>
             <RecyclePool x:Key='pool'/>
             <RecyclingElementFactory x:Key='factory' RecyclePool='{{StaticResource pool}}'>
               <DataTemplate x:Key='item'><Border Width='36' Height='18'/></DataTemplate>
             </RecyclingElementFactory>
           </UserControl.Resources>
           <ItemsRepeater Name='items' ItemTemplate='{{StaticResource factory}}'>
             <ItemsRepeater.Layout><StackLayout Spacing='2'/></ItemsRepeater.Layout>
           </ItemsRepeater>
         </UserControl>"
    ));
    let repeater = root.find_control::<ItemsRepeater>("items").unwrap();
    realize(&repeater, 3);
    assert_eq!(repeater.try_get_element(1).unwrap().bounds().y, 20.0);
    let factory_value = repeater.item_template().unwrap();
    let factory = ferroui_base::metadata::from_markup_value::<Ref<RecyclingElementFactory>>(&Some(
        factory_value,
    ))
    .unwrap();
    assert!(factory.recycle_pool().is_some());
}

#[test]
fn a_shared_layout_keeps_host_specific_state() {
    let _scope = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Resources><StackLayout x:Key='layout' Spacing='3'/></UserControl.Resources>
           <StackPanel>
             <ItemsRepeater Name='first' Layout='{{StaticResource layout}}'><DataTemplate><Border Width='20' Height='10'/></DataTemplate></ItemsRepeater>
             <ItemsRepeater Name='second' Layout='{{StaticResource layout}}'><DataTemplate><Border Width='20' Height='30'/></DataTemplate></ItemsRepeater>
           </StackPanel>
         </UserControl>"
    ));
    let first = root.find_control::<ItemsRepeater>("first").unwrap();
    let second = root.find_control::<ItemsRepeater>("second").unwrap();
    assert_eq!(first.layout(), second.layout());
    realize(&first, 3);
    realize(&second, 3);
    assert_eq!(first.try_get_element(1).unwrap().bounds().y, 13.0);
    assert_eq!(second.try_get_element(1).unwrap().bounds().y, 33.0);
}

#[test]
fn layout_panel_uses_children_as_its_content() {
    let _scope = xaml_test_base();
    let panel = load_as::<Ref<LayoutPanel>>(&format!(
        "<LayoutPanel {NS} Padding='4' BorderThickness='2' CornerRadius='3'>
           <LayoutPanel.Layout><StackLayout Orientation='Horizontal' Spacing='5'/></LayoutPanel.Layout>
           <Border Width='20' Height='10'/><Border Width='30' Height='10'/>
         </LayoutPanel>"
    ));
    assert_eq!(panel.children().count(), 2);
    assert_eq!(panel.padding(), Thickness::uniform(4.0));
    panel.measure(Size::new(100.0, 50.0));
    panel.arrange(Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(panel.children().get(0).bounds().x, 6.0);
    assert_eq!(panel.children().get(1).bounds().x, 31.0);
}

#[test]
fn scroll_host_accepts_native_scroll_viewer_content() {
    let _scope = xaml_test_base();
    let host = load_as::<Ref<ItemsRepeaterScrollHost>>(&format!(
        "<ItemsRepeaterScrollHost {NS} HorizontalAnchorRatio='0.25' VerticalAnchorRatio='0.75'>
           <ScrollViewer><ItemsRepeater/></ScrollViewer>
         </ItemsRepeaterScrollHost>"
    ));
    assert_eq!(host.horizontal_anchor_ratio(), 0.25);
    assert_eq!(host.vertical_anchor_ratio(), 0.75);
    let viewer = host.scroll_viewer().unwrap();
    assert!(Control::from_boxed(&viewer.content().unwrap())
        .unwrap()
        .is::<ItemsRepeater>());
}

#[test]
fn native_controls_in_a_markup_items_source_are_realized_without_wrappers() {
    let _scope = xaml_test_base();
    let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
        "<ItemsRepeater {NS} xmlns:col='using:System.Collections'>
           <ItemsRepeater.ItemsSource><col:ArrayList>
             <Border Name='original' Width='32' Height='18'/>
           </col:ArrayList></ItemsRepeater.ItemsSource>
         </ItemsRepeater>"
    ));
    repeater.measure(Size::new(320.0, 240.0));
    repeater.arrange(Rect::new(0.0, 0.0, 320.0, 240.0));
    let original = repeater.items_source_view().unwrap().get_at(0).unwrap();
    let control = Control::from_boxed(&original).unwrap();
    assert_eq!(repeater.try_get_element(0), Some(control));
}

#[test]
fn compiled_item_binding_infers_the_typed_source_element_and_tracks_changes() {
    use crate::support::emitter::{Row, Table};

    let _scope = xaml_test_base();
    let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
        "<ItemsRepeater {NS} xmlns:t='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
           x:DataType='t:Table' ItemsSource='{{CompiledBinding Rows}}'>
           <DataTemplate><Border Width='32' Height='18' Tag='{{CompiledBinding Name}}'/></DataTemplate>
         </ItemsRepeater>"
    ));
    let table = Table::new();
    repeater.set_data_context(Some(std::rc::Rc::new(table.clone())));
    repeater.measure(Size::new(320.0, 240.0));
    repeater.arrange(Rect::new(0.0, 0.0, 320.0, 240.0));
    assert_eq!(repeater.items_source_view().unwrap().count(), 2);
    assert_eq!(
        repeater
            .try_get_element(0)
            .unwrap()
            .tag()
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("first")
    );
    table.rows().insert(0, Row::new("inserted"));
    repeater.measure(Size::new(320.0, 240.0));
    repeater.arrange(Rect::new(0.0, 0.0, 320.0, 240.0));
    assert_eq!(repeater.items_source_view().unwrap().count(), 3);
    assert_eq!(
        repeater
            .try_get_element(0)
            .unwrap()
            .tag()
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("inserted")
    );
}

#[test]
fn recycle_pool_attached_reuse_key_round_trips_in_markup() {
    let _scope = xaml_test_base();
    let border =
        load_as::<Ref<Border>>(&format!("<Border {NS} RecyclePool.ReuseKey='wide-item'/>"));
    assert_eq!(RecyclePool::get_reuse_key(&border.upcast()), "wide-item");
}

#[test]
fn selection_service_loads_native_source_and_index_path() {
    use ferroui_controls::repeater::selection::{IndexPath, SelectionModel};
    use std::rc::Rc;

    let _scope = xaml_test_base();
    let model = load_as::<Rc<SelectionModel>>(&format!(
        "<r:SelectionModel {NS} xmlns:r='using:FerroUI.Controls.Repeater' xmlns:col='using:System.Collections' SingleSelect='False'>
           <r:SelectionModel.Source><col:ArrayList><x:String>first</x:String><x:String>second</x:String></col:ArrayList></r:SelectionModel.Source>
           <r:SelectionModel.SelectedIndex><r:IndexPath><x:Arguments><x:Int32>1</x:Int32></x:Arguments></r:IndexPath></r:SelectionModel.SelectedIndex>
         </r:SelectionModel>"
    ));
    assert!(!model.single_select());
    assert_eq!(model.source().unwrap().count(), 2);
    assert_eq!(model.selected_index(), Some(IndexPath::create_from(1)));
    assert_eq!(
        model
            .selected_item()
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("second")
    );
}

#[test]
fn repeater_conversions_are_registered_on_subsequent_ui_threads() {
    // The process-wide metadata registration can have happened on a thread
    // that has already exited. Each new UI thread still needs its conversions.
    crate::register_types();
    std::thread::spawn(|| {
        use ferroui_controls::repeater::selection::SelectionModel;
        use std::rc::Rc;

        let _scope = xaml_test_base();
        let factory = load_as::<Ref<RecyclingElementFactory>>(&format!(
            "<RecyclingElementFactory {NS}><DataTemplate x:Key='item'><Border Width='19'/></DataTemplate></RecyclingElementFactory>"
        ));
        assert_eq!(factory.templates().count(), 1);
        assert_eq!(factory.templates().try_get_value(&"item".to_owned()).unwrap().build(&None).unwrap().width(), 19.0);

        let model = SelectionModel::new();
        model.set_source(Some(ItemsSource::from_values(["first".to_owned(), "second".to_owned()])));
        model.select(1).unwrap();
        let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
            "<ItemsRepeater {NS} ItemsSource='{{Binding SelectedItems}}'><DataTemplate><Border Width='20' Height='10'/></DataTemplate></ItemsRepeater>"
        ));
        repeater.set_data_context(Some(Rc::new(model)));
        repeater.measure(Size::new(100.0, 100.0));
        let view = repeater.items_source_view().unwrap();
        assert_eq!(view.count(), 1);
        assert_eq!(view.get_at(0).unwrap().downcast_ref::<String>().map(String::as_str), Some("second"));
    }).join().unwrap();
}

#[test]
fn items_source_view_uses_its_native_class_name_and_preserves_binding_identity() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_controls::ItemsSourceView;
    use std::rc::Rc;

    let _scope = xaml_test_base();
    let view = load_as::<Rc<ItemsSourceView>>(&format!(
        "<ItemsSourceView {NS} xmlns:col='using:System.Collections'><x:Arguments><col:ArrayList><x:String>first</x:String></col:ArrayList></x:Arguments></ItemsSourceView>"
    ));
    let boxed: ferroui_base::BoxedValue = Rc::new(view.clone());
    assert_eq!(
        ValueTypes::to_display_string(Some(&boxed)),
        "FerroUI.Controls.ItemsSourceView"
    );
    let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
        "<ItemsRepeater {NS} ItemsSource='{{Binding}}'><DataTemplate><Border Width='20' Height='10'/></DataTemplate></ItemsRepeater>"
    ));
    repeater.set_data_context(Some(boxed));
    repeater.measure(Size::new(100.0, 100.0));
    assert!(Rc::ptr_eq(&repeater.items_source_view().unwrap(), &view));
    assert_eq!(view.count(), 1);
}

#[test]
fn repeater_runtime_types_have_native_markup_names() {
    use ferroui_base::data::core::ValueTypes;

    let _scope = xaml_test_base();
    for name in [
        "ItemsRepeater",
        "ItemsRepeaterScrollHost",
        "LayoutPanel",
        "ElementFactory",
        "RecyclingElementFactory",
        "RecyclePool",
        "VirtualizingLayout",
        "NonVirtualizingLayout",
        "VirtualizingLayoutContext",
        "NonVirtualizingLayoutContext",
        "StackLayout",
        "StackLayoutState",
        "UniformGridLayout",
        "UniformGridLayoutState",
        "FlowLayout",
        "FlowLayoutState",
        "LinedFlowLayout",
        "ItemCollectionTransitionProvider",
        "LinedFlowLayoutItemCollectionTransitionProvider",
    ] {
        let value = crate::support::loader::load(&format!("<{name} {NS}/>"));
        let object = ValueTypes::as_object(&*value).unwrap();
        assert_eq!(
            object.get_type().full_name(),
            format!("FerroUI.Controls.{name}")
        );
        assert_eq!(
            ValueTypes::to_display_string(Some(&value)),
            format!("FerroUI.Controls.{name}")
        );
    }
}

#[test]
fn repeater_service_metadata_has_native_names() {
    use ferroui_base::metadata::MarkupType;
    use ferroui_controls::repeater::selection::*;
    use ferroui_controls::repeater::*;
    use std::any::TypeId;
    use std::rc::Rc;

    let _scope = xaml_test_base();
    macro_rules! name {
        ($handle:ty, $expected:expr) => {
            assert_eq!(
                MarkupType::find_by_handle(TypeId::of::<$handle>())
                    .unwrap()
                    .full_name(),
                $expected
            );
        };
    }
    name!(IndexPath, "FerroUI.Controls.Repeater.IndexPath");
    name!(
        Rc<SelectionModel>,
        "FerroUI.Controls.Repeater.SelectionModel"
    );
    name!(
        SelectionModelSelectionChangedEventArgs,
        "FerroUI.Controls.Repeater.SelectionModelSelectionChangedEventArgs"
    );
    name!(
        Rc<SelectionModelChildrenRequestedEventArgs>,
        "FerroUI.Controls.Repeater.SelectionModelChildrenRequestedEventArgs"
    );
    name!(
        ElementFactoryGetArgs,
        "FerroUI.Controls.Repeater.ElementFactoryGetArgs"
    );
    name!(
        ElementFactoryRecycleArgs,
        "FerroUI.Controls.Repeater.ElementFactoryRecycleArgs"
    );
    name!(
        SelectTemplateEventArgs,
        "FerroUI.Controls.Repeater.SelectTemplateEventArgs"
    );
    name!(
        ItemsRepeaterElementPreparedEventArgs,
        "FerroUI.Controls.ItemsRepeaterElementPreparedEventArgs"
    );
    name!(
        ItemsRepeaterElementClearingEventArgs,
        "FerroUI.Controls.ItemsRepeaterElementClearingEventArgs"
    );
    name!(
        ItemsRepeaterElementIndexChangedEventArgs,
        "FerroUI.Controls.ItemsRepeaterElementIndexChangedEventArgs"
    );
    name!(
        LinedFlowLayoutItemsInfoRequestedEventArgs,
        "FerroUI.Controls.LinedFlowLayoutItemsInfoRequestedEventArgs"
    );
}

#[test]
fn repeater_markup_activation_requires_native_constructor_arguments() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_controls::RepeaterAutomationPeer;

    let _scope = xaml_test_base();
    assert!(crate::support::loader::try_load(&format!("<ItemsSourceView {NS}/>")).is_err());
    assert!(crate::support::loader::try_load(&format!(
        "<p:RepeaterAutomationPeer {NS} xmlns:p='using:FerroUI.Automation.Peers'/>"
    ))
    .is_err());
    let peer = load_as::<Ref<RepeaterAutomationPeer>>(&format!(
        "<p:RepeaterAutomationPeer {NS} xmlns:p='using:FerroUI.Automation.Peers'><x:Arguments><ItemsRepeater/></x:Arguments></p:RepeaterAutomationPeer>"
    ));
    assert_eq!(
        peer.get_type().full_name(),
        "FerroUI.Automation.Peers.RepeaterAutomationPeer"
    );
    assert!(peer.owner().get_type().name() == "ItemsRepeater");
    let model = crate::support::loader::load(&format!(
        "<r:SelectionModel {NS} xmlns:r='using:FerroUI.Controls.Repeater'/>"
    ));
    assert_eq!(
        ValueTypes::to_display_string(Some(&model)),
        "FerroUI.Controls.Repeater.SelectionModel"
    );
}
