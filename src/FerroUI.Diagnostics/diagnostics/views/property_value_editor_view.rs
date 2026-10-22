//! Port of `Diagnostics/Views/PropertyValueEditorView.cs`.
//!
//! The editor of the value of a property in the property grid: a check box, a number box, a
//! color, a brush, an image or geometry preview, a list of the members of an enumeration, or a
//! text box, by the type of the property. The managed original asks reflection what a type is
//! (`IsEnum`, `TypeCode`, implemented interfaces, type converters and `Parse` methods); the port
//! asks the value types and the markup metadata of the framework the same questions.

use ferroui_base::{StyledElementImplExt, VisualImplExt};
use crate::diagnostics::controls::{BrushEditor, CommitTextBox};
use crate::diagnostics::view_models::{PropertiesViewItem, PropertyViewModelRef};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingError, BindingMode, BindingOperations};
use ferroui_base::input::{
    Cursor, InputElement, InputElementImpl, PointerPressedEventArgs, StandardCursorType,
};
use ferroui_base::interactivity::{Interactive, InteractiveImpl};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, Geometry, IBrush, IImage, Stretch};
use ferroui_base::metadata::{MarkupType, MarkupTypeKind};
use ferroui_base::reactive::{CompositeDisposable, IDisposable};
use ferroui_base::utilities::{CultureInfo, Decimal, NumberFormatInfo, NumberStyles};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, VisualImpl, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::{FlyoutBase, SelectingItemsControl, TemplatedControlImpl, ToggleButton};
use ferroui_controls::shapes::{Ellipse, Path, Shape};
use ferroui_controls::{
    Border, CheckBox, ComboBox, ContentControlImpl, Control, ControlImpl, DataValidationErrors, Flyout, Image,
    ItemsSource, NumericUpDown, StackPanel, TextBlock, TextBox, ToolTip, UserControl,
};
use ferroui_controls_color_picker::{AlphaComponentPosition, ColorView};
use ferroui_markup::data::Binding;
use ferroui_markup_xaml::converters::ColorToBrushConverter;
use std::cell::OnceCell;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// `ImageIcon`: parsed on first use, on the UI thread.
    static IMAGE_ICON: OnceCell<Ref<Geometry>> = const { OnceCell::new() };
    /// `GeometryIcon`: parsed on first use, on the UI thread.
    static GEOMETRY_ICON: OnceCell<Ref<Geometry>> = const { OnceCell::new() };
    /// `Color2Brush`.
    static COLOR2_BRUSH: OnceCell<Rc<ColorToBrushConverter>> = const { OnceCell::new() };
}

const IMAGE_ICON_DATA: &str = "M12.25 6C8.79822 6 6 8.79822 6 12.25V35.75C6 37.1059 6.43174 38.3609 7.16525 39.3851L21.5252 25.0251C22.8921 23.6583 25.1081 23.6583 26.475 25.0251L40.8348 39.385C41.5683 38.3608 42 37.1058 42 35.75V12.25C42 8.79822 39.2018 6 35.75 6H12.25ZM34.5 17.5C34.5 19.7091 32.7091 21.5 30.5 21.5C28.2909 21.5 26.5 19.7091 26.5 17.5C26.5 15.2909 28.2909 13.5 30.5 13.5C32.7091 13.5 34.5 15.2909 34.5 17.5ZM39.0024 41.0881L24.7072 26.7929C24.3167 26.4024 23.6835 26.4024 23.293 26.7929L8.99769 41.0882C9.94516 41.6667 11.0587 42 12.25 42H35.75C36.9414 42 38.0549 41.6666 39.0024 41.0881Z";

const GEOMETRY_ICON_DATA: &str = "M23.25 15.5H30.8529C29.8865 8.99258 24.2763 4 17.5 4C10.0442 4 4 10.0442 4 17.5C4 24.2763 8.99258 29.8865 15.5 30.8529V23.25C15.5 18.9698 18.9698 15.5 23.25 15.5ZM23.25 18C20.3505 18 18 20.3505 18 23.25V38.75C18 41.6495 20.3505 44 23.25 44H38.75C41.6495 44 44 41.6495 44 38.75V23.25C44 20.3505 41.6495 18 38.75 18H23.25Z";

fn image_icon() -> Ref<Geometry> {
    IMAGE_ICON.with(|icon| {
        icon.get_or_init(|| Geometry::parse(IMAGE_ICON_DATA).expect("the image icon is valid path data")).clone()
    })
}

fn geometry_icon() -> Ref<Geometry> {
    GEOMETRY_ICON.with(|icon| {
        icon.get_or_init(|| Geometry::parse(GEOMETRY_ICON_DATA).expect("the geometry icon is valid path data")).clone()
    })
}

fn color2_brush() -> Rc<ColorToBrushConverter> {
    COLOR2_BRUSH.with(|converter| converter.get_or_init(ColorToBrushConverter::new).clone())
}

/// The editor of the value of a property: its content is the control that edits a value of the
/// type of the property (its data context), bound to the value of the property.
#[repr(C)]
pub struct PropertyValueEditorView {
    base: UserControl,
    cleanup: CompositeDisposable,
}

ferro_class!(PropertyValueEditorView: UserControl);
ferro_impl_classes!(
    PropertyValueEditorView: FerroObjectImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(PropertyValueEditorView { new: PropertyValueEditorView::new });

impl StyledElementImpl for PropertyValueEditorView {
    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);

        let content = this.update_control().map(Control::boxed);
        this.set_content(content);
    }
}

impl VisualImpl for PropertyValueEditorView {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        this.cleanup.clear();
    }
}

/// `ImplementsInterface<TInterface>(type)`: the type is the contract (by its handle, or its
/// nullable handle), or a type assignable to it.
fn implements_interface<I: 'static>(type_: ValueType) -> bool {
    let interface_type = ValueType::of::<I>();
    let nullable_interface_type = ValueType::of::<Option<I>>();
    type_ == interface_type || type_ == nullable_interface_type || ValueTypes::is_assignable(type_, interface_type)
}

/// `type.IsEnum`: an enumeration with markup metadata.
fn is_enum(type_: ValueType) -> bool {
    MarkupType::find_by_handle(type_.id()).is_some_and(|markup| markup.kind == MarkupTypeKind::Enum)
}

/// `IsValidNumeric`: an integral number or a single-precision number (`TypeCode` `Byte` to
/// `UInt64` and `Single`), or the nullable form of one. Doubles and decimals are not edited as
/// numbers (the managed original notes that infinity and NaN do not work with `NumericUpDown`).
fn is_valid_numeric(type_: ValueType) -> bool {
    if is_enum(type_) {
        return false;
    }
    type_.is::<u8>()
        || type_.is::<i16>()
        || type_.is::<i32>()
        || type_.is::<i64>()
        || type_.is::<i8>()
        || type_.is::<f32>()
        || type_.is::<u16>()
        || type_.is::<u32>()
        || type_.is::<u64>()
        || type_.is::<Option<u8>>()
        || type_.is::<Option<i16>>()
        || type_.is::<Option<i32>>()
        || type_.is::<Option<i64>>()
        || type_.is::<Option<i8>>()
        || type_.is::<Option<f32>>()
        || type_.is::<Option<u16>>()
        || type_.is::<Option<u32>>()
        || type_.is::<Option<u64>>()
}

/// A number as the managed original formats it into text.
fn format_number(value: f64) -> String {
    if value == f64::INFINITY {
        "\u{221e}".to_string()
    } else if value == f64::NEG_INFINITY {
        "-\u{221e}".to_string()
    } else {
        format!("{value}")
    }
}

impl PropertyValueEditorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), cleanup: CompositeDisposable::new() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `Property => (PropertyViewModel?)DataContext`: the property of the data context, which is
    /// a property view model or the row of the property grid of one.
    fn property(&self) -> Option<PropertyViewModelRef> {
        let data_context = self.data_context()?;
        PropertyViewModelRef::from_boxed(&data_context)
            .or_else(|| PropertiesViewItem::from_boxed(&data_context).and_then(|item| item.property()))
    }

    fn update_control(&self) -> Option<Ref<Control>> {
        self.cleanup.clear();

        let property = self.property()?;
        let property_type = property.property_type();

        if property_type.is::<bool>() || property_type.is::<Option<bool>>() {
            let check_box = CheckBox::new();
            check_box.classes().add("editor");
            self.create_control(&property, &check_box, ToggleButton::is_checked_property().as_property(), None, None);
            return Some(check_box.upcast());
        }

        // TODO (from the original): Infinity, NaN not working with NumericUpDown.
        if is_valid_numeric(property_type) {
            let n = NumericUpDown::new();
            n.classes().add("editor");
            // `init`.
            n.set_increment(Decimal::from(1));
            let number_format = NumberFormatInfo::new();
            number_format.set_number_decimal_digits(0);
            n.set_number_format(Some(Rc::new(number_format)));
            n.set_parsing_number_style(NumberStyles::INTEGER);

            self.create_control(
                &property,
                &n,
                NumericUpDown::value_property().as_property(),
                Some(Rc::new(ValueToDecimalConverter)),
                Some(ReadonlyProperty::NumericUpDown),
            );
            return Some(n.upcast());
        }

        if property_type.is::<Color>() {
            let el = Ellipse::new();
            el.set_width(12.0);
            el.set_height(12.0);
            el.set_vertical_alignment(VerticalAlignment::Center);

            let binding = Binding::with_path("Value")
                .with_source(Some(property.boxed()))
                .with_converter(color2_brush() as Rc<dyn IValueConverter>);
            self.add_cleanup(el.bind_binding(Shape::fill_property().as_property(), &*binding));

            let tbl = TextBlock::new();
            tbl.set_vertical_alignment(VerticalAlignment::Center);

            let binding = Binding::with_path("Value").with_source(Some(property.boxed()));
            self.add_cleanup(tbl.bind_binding(TextBlock::text_property().as_property(), &*binding));

            let sp = StackPanel::new();
            sp.set_orientation(Orientation::Horizontal);
            sp.set_spacing(2.0);
            sp.children().add(el);
            sp.children().add(tbl);
            sp.set_background(Some(Brushes::transparent() as Rc<dyn IBrush>));
            sp.set_cursor(Some(Cursor::new(StandardCursorType::Hand)));
            sp.classes().add("editor");
            sp.set_is_enabled(!property.is_readonly());

            let cv = ColorView::new();
            // Always match XAML.
            cv.set_hex_input_alpha_position(AlphaComponentPosition::Leading);

            let binding = Binding::with_path_and_mode("Value", BindingMode::TwoWay)
                .with_source(Some(property.boxed()))
                .with_converter(color2_brush() as Rc<dyn IValueConverter>);
            self.add_cleanup(cv.bind_binding(ColorView::color_property().as_property(), &*binding));

            let flyout = Flyout::new();
            flyout.set_content(Some(Control::boxed(cv)));
            FlyoutBase::set_attached_flyout(&sp, flyout);

            let weak = sp.downgrade();
            sp.add_handler(InputElement::pointer_pressed_event(), move |_: &Interactive, _: &PointerPressedEventArgs| {
                if let Some(sp) = weak.upgrade() {
                    FlyoutBase::show_attached_flyout(&sp);
                }
            });

            return Some(sp.upcast());
        }

        if implements_interface::<Rc<dyn IBrush>>(property_type) {
            let brush_editor = BrushEditor::new();
            brush_editor.classes().add("editor");
            self.create_control(&property, &brush_editor, BrushEditor::brush_property().as_property(), None, None);
            return Some(brush_editor.upcast());
        }

        let is_image = implements_interface::<Rc<dyn IImage>>(property_type);
        let is_geometry = property_type.is::<Ref<Geometry>>() || property_type.is::<Option<Ref<Geometry>>>();

        if is_image || is_geometry {
            let tbl = TextBlock::new();
            tbl.set_vertical_alignment(VerticalAlignment::Center);

            // `valueObservable.Select(..)`: the text of the size of the image or geometry.
            let binding = Binding::with_path("Value")
                .with_source(Some(property.boxed()))
                .with_converter(Rc::new(SizeTextConverter) as Rc<dyn IValueConverter>);
            self.add_cleanup(tbl.bind_binding(TextBlock::text_property().as_property(), &*binding));

            let icon = Path::new();
            icon.set_data(if is_image { image_icon() } else { geometry_icon() });
            icon.set_fill(Some(Brushes::gray() as Rc<dyn IBrush>));
            icon.set_width(12.0);
            icon.set_height(12.0);
            icon.set_stretch(Stretch::Uniform);
            icon.set_vertical_alignment(VerticalAlignment::Center);

            let sp = StackPanel::new();
            sp.set_background(Some(Brushes::transparent() as Rc<dyn IBrush>));
            sp.set_orientation(Orientation::Horizontal);
            sp.set_spacing(2.0);
            sp.children().add(icon);
            sp.children().add(tbl);
            sp.classes().add("editor");

            if is_image {
                let preview_image = Image::new();
                preview_image.set_stretch(Stretch::Uniform);
                preview_image.set_width(300.0);
                preview_image.set_height(300.0);

                let binding = Binding::with_path("Value").with_source(Some(property.boxed()));
                self.add_cleanup(preview_image.bind_binding(Image::source_property().as_property(), &*binding));

                ToolTip::set_tip(&sp, Some(Control::boxed(preview_image)));
            } else {
                let preview_shape = Path::new();
                preview_shape.set_stretch(Stretch::Uniform);
                preview_shape.set_fill(Some(Brushes::white() as Rc<dyn IBrush>));
                preview_shape.set_vertical_alignment(VerticalAlignment::Center);
                preview_shape.set_horizontal_alignment(HorizontalAlignment::Center);

                let binding = Binding::with_path("Value").with_source(Some(property.boxed()));
                self.add_cleanup(preview_shape.bind_binding(Path::data_property().as_property(), &*binding));

                let border = Border::new();
                border.set_child(preview_shape);
                border.set_width(300.0);
                border.set_height(300.0);
                ToolTip::set_tip(&sp, Some(Control::boxed(border)));
            }

            return Some(sp.upcast());
        }

        if is_enum(property_type) {
            let c = ComboBox::new();
            c.classes().add("editor");
            // `init`: `Enum.GetValues(propertyType)`.
            let members = MarkupType::find_by_handle(property_type.id()).map(|markup| markup.enum_members).unwrap_or(&[]);
            c.set_items_source(Some(ItemsSource::from_items(members.iter().map(|member| Some((member.get)())))));
            self.create_control(
                &property,
                &c,
                SelectingItemsControl::selected_item_property().as_property(),
                None,
                None,
            );
            return Some(c.upcast());
        }

        let tb = CommitTextBox::new();
        // Deviation (DEVIATIONS.md, Developer tools): every editor has the class `editor`, which
        // the sheet of the tools (DevToolsStyles.xaml) gives one compact form.
        tb.classes().add("editor");
        // `init`.
        tb.set_placeholder_text(Some("(null)"));
        self.create_control(
            &property,
            &tb,
            CommitTextBox::committed_text_property().as_property(),
            Some(Rc::new(TextToValueConverter)),
            Some(ReadonlyProperty::TextBox),
        );

        let is_read_only =
            tb.is_read_only() || property_type.is_object() || !StringConversionHelper::can_convert_from_string(property_type);
        tb.set_is_read_only(is_read_only);

        if !tb.is_read_only() {
            // `tb.GetObservable(TextBox.TextProperty).Subscribe(..)`: the current text, then every
            // change of it.
            let validate = move |tb: &TextBox, t: Option<String>| {
                let result = match t {
                    Some(t) => StringConversionHelper::from_string(&t, property_type).map(|_| ()),
                    None => Ok(()),
                };
                match result {
                    Ok(()) => DataValidationErrors::clear_errors(tb),
                    Err(error) => DataValidationErrors::set_error(tb, Some(&error)),
                }
            };
            let text_box: &TextBox = &tb;
            validate(text_box, tb.text());
            let weak = tb.downgrade();
            self.cleanup.add(tb.property_changed(move |e: &FerroPropertyChangedEventArgs<'_>| {
                if e.property() == TextBox::text_property().as_property() {
                    if let Some(tb) = weak.upgrade() {
                        let text_box: &TextBox = &tb;
                        validate(text_box, tb.text());
                    }
                }
            }));
        }

        Some(tb.upcast())
    }

    fn add_cleanup(&self, expression: Rc<dyn ferroui_base::data::BindingExpressionBase>) {
        self.cleanup.add(expression as Rc<dyn IDisposable>);
    }

    /// `CreateControl<TControl>`: binds `value_property` of `control` to the value of the
    /// property (two-way unless the property is read-only), and makes the control read-only or
    /// disabled for a read-only property. The control is created, and initialized, by the
    /// caller.
    fn create_control(
        &self,
        property: &PropertyViewModelRef,
        control: &Control,
        value_property: &'static FerroProperty,
        converter: Option<Rc<dyn IValueConverter>>,
        readonly_property: Option<ReadonlyProperty>,
    ) {
        let binding_mode = if property.is_readonly() { BindingMode::OneWay } else { BindingMode::TwoWay };

        let binding = Binding::with_path_and_mode("Value", binding_mode)
            .with_source(Some(property.boxed()))
            .with_converter(converter.unwrap_or_else(|| Rc::new(ValueConverter) as Rc<dyn IValueConverter>))
            .with_converter_parameter(Some(Rc::new(property.property_type()) as BoxedValue));
        self.add_cleanup(control.bind_binding(value_property, &*binding));

        match readonly_property {
            Some(ReadonlyProperty::NumericUpDown) => {
                control.set_value(NumericUpDown::is_read_only_property(), property.is_readonly())
            }
            Some(ReadonlyProperty::TextBox) => control.set_value(TextBox::is_read_only_property(), property.is_readonly()),
            None => control.set_is_enabled(!property.is_readonly()),
        }
    }
}

/// The read-only property of a control `CreateControl` sets (`readonlyProperty`).
#[derive(Clone, Copy)]
enum ReadonlyProperty {
    NumericUpDown,
    TextBox,
}

/// The type a converter converts back to: the type of the property, the parameter of the
/// binding (the target type the binding gives is that of the value of the view model).
fn parameter_type(parameter: Option<&BoxedValue>, target_type: ValueType) -> ValueType {
    parameter.and_then(|parameter| parameter.downcast_ref::<ValueType>().copied()).unwrap_or(target_type)
}

/// `ValueConverter`: passes the values through.
struct ValueConverter;

impl IValueConverter for ValueConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.cloned())
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.cloned())
    }
}

/// `ValueToDecimalConverter`: a number as a decimal, and a decimal back to the type of the
/// property.
struct ValueToDecimalConverter;

impl ValueToDecimalConverter {
    /// `System.Convert.ToDecimal(value)`.
    fn to_decimal(value: &BoxedValue) -> Option<Decimal> {
        let value = ValueTypes::normalize(value.clone())?;
        macro_rules! integers {
            ($($ty:ty),*) => {
                $(
                    if let Some(v) = value.downcast_ref::<$ty>() {
                        return Some(Decimal::from(*v));
                    }
                )*
            };
        }
        integers!(u8, i8, i16, u16, i32, u32, i64, u64);
        if let Some(v) = value.downcast_ref::<f32>() {
            return Decimal::try_from_f32(*v);
        }
        if let Some(v) = value.downcast_ref::<f64>() {
            return Decimal::try_from_f64(*v);
        }
        value.downcast_ref::<Decimal>().copied()
    }

    /// `System.Convert.ChangeType(value, targetType)`: a decimal as a number of `target_type`.
    fn change_type(value: Decimal, target_type: ValueType) -> Option<BoxedValue> {
        let target_type = ValueTypes::nullable_inner(target_type).unwrap_or(target_type);
        let integer = value.round_to_integer();
        macro_rules! integers {
            ($($ty:ty),*) => {
                $(
                    if target_type.is::<$ty>() {
                        return <$ty>::try_from(integer).ok().map(|v| Rc::new(v) as BoxedValue);
                    }
                )*
            };
        }
        integers!(u8, i8, i16, u16, i32, u32, i64, u64);
        if target_type.is::<f32>() {
            return Some(Rc::new(value.to_f64() as f32));
        }
        if target_type.is::<f64>() {
            return Some(Rc::new(value.to_f64()));
        }
        None
    }
}

impl IValueConverter for ValueToDecimalConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // `Convert.ToDecimal(null)` is zero.
        let decimal = match value {
            Some(value) => Self::to_decimal(value)
                .ok_or_else(|| BindingError::message("The value cannot be converted to a decimal."))?,
            None => Decimal::from(0),
        };
        Ok(Some(Rc::new(decimal)))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let target_type = parameter_type(parameter, target_type);
        let Some(decimal) = value.and_then(Self::to_decimal) else {
            return Ok(None);
        };
        Self::change_type(decimal, target_type)
            .map(Some)
            .ok_or_else(|| BindingError::message("The value cannot be converted to the type of the property."))
    }
}

/// `TextToValueConverter`: a value as its text, and text back to a value of the type of the
/// property (nothing for text that does not convert).
struct TextToValueConverter;

impl IValueConverter for TextToValueConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.map(|value| Rc::new(StringConversionHelper::to_string(value)) as BoxedValue))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let target_type = parameter_type(parameter, target_type);
        let Some(s) = value.and_then(|value| value.downcast_ref::<String>()) else {
            return Ok(None);
        };

        match StringConversionHelper::from_string(s, target_type) {
            Ok(value) => Ok(value),
            Err(_) => Ok(Some(BindingOperations::do_nothing())),
        }
    }
}

/// The text of the size of an image or a geometry (`img.Size` / `geom.Bounds`), `(null)` for
/// anything else.
struct SizeTextConverter;

impl IValueConverter for SizeTextConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let value = value.and_then(|value| ValueTypes::normalize(value.clone()));
        let text = match value {
            Some(value) => {
                if let Some(img) = value.downcast_ref::<Rc<dyn IImage>>() {
                    let size = img.size();
                    format!("{} x {}", format_number(size.width), format_number(size.height))
                } else if let Some(geom) = value.downcast_ref::<Ref<Geometry>>() {
                    let bounds = geom.bounds();
                    format!("{} x {}", format_number(bounds.width), format_number(bounds.height))
                } else {
                    "(null)".to_string()
                }
            }
            None => "(null)".to_string(),
        };
        Ok(Some(Rc::new(text)))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(BindingOperations::do_nothing()))
    }
}

/// `StringConversionHelper`: the conversion of values to and from text. The managed original
/// asks the type converter of a type (`TypeDescriptor.GetConverter`) and its `Parse` methods;
/// the port asks the value types and the conversion from text the markup metadata of a type
/// states (`parse:`, the counterpart of both).
struct StringConversionHelper;

impl StringConversionHelper {
    /// Whether text converts to a value of `type_`.
    fn can_convert_from_string(type_: ValueType) -> bool {
        let type_ = ValueTypes::nullable_inner(type_).unwrap_or(type_);
        if type_.is_string() || type_.is::<Option<String>>() {
            return true;
        }
        if type_.is::<bool>()
            || type_.is::<char>()
            || type_.is::<u8>()
            || type_.is::<i8>()
            || type_.is::<i16>()
            || type_.is::<u16>()
            || type_.is::<i32>()
            || type_.is::<u32>()
            || type_.is::<i64>()
            || type_.is::<u64>()
            || type_.is::<f32>()
            || type_.is::<f64>()
            || type_.is::<Decimal>()
        {
            return true;
        }

        // The conversion of the nearest class that states one (a converter attribute is
        // inherited).
        if let Some(class) = ValueTypes::class_of(type_) {
            let mut current = Some(class);
            while let Some(class) = current {
                if class.markup().is_some_and(|markup| markup.parse.is_some()) {
                    return true;
                }
                current = class.base_type();
            }
            return false;
        }

        MarkupType::find_by_handle(type_.id())
            .or_else(|| MarkupType::find_by_nullable_handle(type_.id()))
            .is_some_and(|markup| markup.parse.is_some() || markup.kind == MarkupTypeKind::Enum)
    }

    /// The text of a value (`ConvertToInvariantString`, or `ToString()`).
    fn to_string(o: &BoxedValue) -> String {
        ValueTypes::to_display_string(Some(o))
    }

    /// The value of `type_` the text `s` converts to; an error for text that does not convert.
    fn from_string(s: &str, type_: ValueType) -> Result<Option<BoxedValue>, BindingError> {
        let text: BoxedValue = Rc::new(s.to_string());

        if let Some(value) = ValueTypes::try_convert(Some(&text), type_) {
            return Ok(value);
        }
        if let Some(value) = ValueTypes::try_parse_text(&text, type_) {
            return Ok(Some(value));
        }
        if let Some(inner) = ValueTypes::nullable_inner(type_) {
            if let Some(value) = ValueTypes::try_parse_text(&text, inner) {
                return Ok(ValueTypes::try_convert(Some(&value), type_).flatten().or(Some(value)));
            }
        }

        Err(BindingError::message(format!("'{s}' is not a valid value of {}.", type_.name())))
    }
}
