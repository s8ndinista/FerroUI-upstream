# WinUI theme port: deviations from WinUI 3

Every place where the WinUI theme of FerroUI (`src/FerroUI.Themes.WinUI`) differs from the
WinUI 3 source it is ported from (microsoft-ui-xaml, commit
`7e4b0c2096be402f51cc8f7d203ebf67cff183d0`), one entry each: the source file and the key or
element, what WinUI does, what the port does, and why. The design is in
[../winui-theme.md](../winui-theme.md); the state of every control is in
[PROGRESS.md](PROGRESS.md) and the item-by-item tracking in [TRACKING.md](TRACKING.md).

Rules for the batches (docs/porting/winui-theme.md, section 7): every batch adds the deviations
of the controls it ports here, in a section `## <FerroUI control>` (the name of its document
under `Controls/`), one table row per deviation with an identifier in the first column
(`| BTN-1 |`), and regenerates the tracking (`scripts/winui-theme/status.py`, which counts the
rows of each section) in the same commit. A general rule that applies to many controls is written
once under "General" and not repeated per control; a control section lists what is particular to
it. Abbreviations: CS = `controls/dev/CommonStyles/`, GX = `dxaml/xcp/dxaml/themes/generic.xaml`.

## General

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| G-1 | `VisualStateManager.VisualStateGroups` of every template | One current state per group, chosen by the code of the control; its storyboard sets values for as long as it is current. | Each state is a nested style of the control theme on the pseudo-classes of the state (`^:pointerover`, `^:checked:pressed`), in the order of the source so that the last matching style is the state WinUI would choose. | FerroUI has no visual state manager; its styles select on pseudo-classes, several of which match at once (a pressed control is also under the pointer). |
| G-2 | `ObjectAnimationUsingKeyFrames` with one discrete frame at time 0, `DoubleAnimation Duration="0"`, `VisualState.Setters` | Animations that set a value at once. | Setters of the nested style. | Same effect without an animation. |
| G-3 | `DoubleAnimation`/`SplineDoubleKeyFrame` to a value over a duration, `BeginTime` | The value is animated when the state is entered; leaving the state ends the animation and restores the base value at once; `BeginTime` delays it. | A setter of the value and a transition of the property on the element, with the duration and the spline of the source; the return to the base value animates too; begin times are not reproduced. | FerroUI animates property changes with transitions; a transition has no begin delay and runs both ways. |
| G-4 | `{ThemeResource X}` | A reference re-evaluated when the theme changes. | `{DynamicResource X}`; in a theme dictionary it resolves in the variant of that dictionary. | The counterpart in FerroUI. |
| G-5 | Theme dictionaries `Default`, `Dark`, `Light`, `HighContrast` | `Dark` is merged into `Default`; `Default` is the dark theme and the fallback; `HighContrast` is selected by the contrast setting of Windows. | `Default` and `Light` keep their keys; `HighContrast` is keyed by `WinUITheme.HighContrast`, a theme variant that inherits `Dark`, which an application requests; it is not selected from the contrast setting of the system. | FerroUI has only the light and dark variants and parses no other variant from text; the actual variant of a top level follows only light or dark of the platform. |
| G-6 | `SystemColor*Color` (named by the `HighContrast` dictionaries) | Colours of the system, from the active contrast theme. | Fixed values in `Resources/SystemResources.xaml`: the standard colours of Windows in `Default`/`Light`, the colours of "High Contrast Black" in `HighContrast`. | FerroUI has no system colours. |
| G-7 | `SystemAccentColor`, `SystemAccentColorDark1..3`, `SystemAccentColorLight1..3` | Colours of the system (`UISettings`). | `Accents/SystemAccentColors`: the accent of the platform settings (default `#0078D7`) and shades computed with the lightness steps of the Fluent theme. | FerroUI's platform settings report one accent colour; Windows computes the shades in its own way. |
| G-8 | `XamlAutoFontFamily` (GX `ContentControlThemeFontFamily`, the text styles) | Resolves to Segoe UI Variable with the optical size of the text. | `Segoe UI Variable Text, Segoe UI, $Default`. | The name is a WinUI convention; the list gives the same font on Windows 11 and the default font where Segoe is absent. |
| G-9 | `FontIcon`, `FontIconSource`, `TextBlock` in `SymbolThemeFontFamily` | Glyphs of the Segoe Fluent Icons font. | `Path` (or `PathIcon`) of the geometry `GlyphGeometry<code point>` of `Resources/Glyphs.xaml`, sized to the ink of the glyph at the font size of the source. | The font is part of Windows, not redistributable and absent on other platforms; the outlines and their sizes are approximations to be calibrated against native WinUI. |
| G-10 | `AnimatedIcon` with an animated visual source (Lottie) | Animated glyphs whose state follows `AnimatedIcon.State`. | The fallback glyph as a static path; the states become its opacity; the `AnimatedIcon.State` setters are left out. | FerroUI has no `AnimatedIcon` and no Lottie player. |
| G-11 | `controls/dev/Materials/Reveal/RevealBrush_themeresources.xaml`, the `*RevealStyle` styles | Reveal brushes and styles, deprecated in WinUI 3 (plain fallbacks). | Waived, except the seven Reveal brushes other documents still name, ported as the plain brushes they are. | FerroUI has no Reveal lighting; the default styles are ported. |
| G-12 | `AcrylicBrush` resources (`controls/dev/Materials/Acrylic/AcrylicBrush_themeresources.xaml`, CS `Deprecated_themeresources_any.xaml`) | A blurred, tinted backdrop brush, with `FallbackColor` where acrylic is not available. | A `SolidColorBrush` of `FallbackColor` (of the tint with its opacity where there is no fallback). | FerroUI has no acrylic brush (its acrylic is a material of a border, not a brush). |
| G-13 | `DesktopAcrylicBackdrop` (`AcrylicBackgroundFillColorDefaultBackdrop`, `MenuFlyoutSystemBackdrop`), Mica of windows | System backdrops of windows and popups. | Waived; windows draw `SolidBackgroundFillColorBaseBrush`, the colour Mica falls back to. | Backdrops are a platform feature an application chooses (the transparency level of a top level). |
| G-14 | `UseSystemFocusVisuals`, `FocusVisualMargin`, `FocusVisualPrimaryBrush/Thickness`, `FocusVisualSecondaryBrush/Thickness` | The framework draws a 2 px outer and a 1 px inner rectangle around the focused element, at its margin, with its corner radius widened by the margin. | The default focus adorner of `AdornerLayer` draws the two rectangles; a control theme with a `FocusVisualMargin` sets its own `FocusAdorner` with that margin; the corner radius is `ControlCornerRadius`, not widened. `UseSystemFocusVisuals="False"` is `FocusAdorner="{x:Null}"`. | FerroUI draws focus visuals as adorners; the adorner template cannot read the corner radius of the adorned control. |
| G-15 | Keyed styles without `BasedOn` (`AccentButtonStyle`) | Applied over the default style of the control, whose setters they do not override stay in effect. | Keyed control themes are based on the default theme of their type (`BasedOn="{StaticResource DefaultButtonStyle}"`). | A FerroUI control theme replaces the default one; the base keeps the layering of WinUI. |
| G-16 | `Style="{StaticResource X}"` in an application | Sets an explicit style. | `Theme="{StaticResource X}"`; the keys are the keys of WinUI. | Control themes are FerroUI's styles of controls. |
| G-17 | `Grid`/`StackPanel` with `BorderBrush`, `BorderThickness`, `CornerRadius`, `Padding` | Panels of WinUI draw borders. | A `Border` of the same name around the panel; state setters target the border. | FerroUI panels have a background only. |
| G-18 | Template-local resources (`Grid.Resources` with `ControlTemplate x:Key`, `Style x:Name`) | Templates and styles that only the template uses. | Keyed control themes of the control document, named `<Control><Name>`. | FerroUI's templates have no local resources of control templates; a keyed theme is the counterpart. |
| G-19 | `AutomationProperties.AccessibilityView`, `ContentTransitions`, `IsTextScaleFactorEnabled`, `Control.IsTemplateFocusTarget`, `x:DeferLoadStrategy`, `x:Load`, `LineStackingStrategy`, `TextLineBounds`, `OpticalMarginAlignment`, `ScrollViewer` modes (`HorizontalScrollMode`, `VerticalScrollMode`, `IsHorizontalRailEnabled`, `IsVerticalRailEnabled`, `ZoomMode`) | Properties of WinUI elements. | Left out. | FerroUI has no such properties (its automation tree and text layout are its own). |
| G-20 | `RecognizesAccessKey` (not in the source) | WinUI's access keys (`AccessKey`) need nothing in templates. | The content presenters of buttons, toggles, check boxes and radio buttons set `RecognizesAccessKey="True"`. | FerroUI's access keys (`_File`) are recognised by the content presenter. |
| G-21 | Durations and key splines as `x:String` resources (`ControlNormalAnimationDuration`, `ControlFastOutSlowInKeySpline`) | Converted where they are used. | Kept as resources; the transitions write the values and name the resource in a comment. | A string resource cannot feed the `TimeSpan` of a transition in FerroUI. |
| G-22 | Theme animations and transitions (`FadeInThemeAnimation`, `PopupThemeTransition`, `*ThemeTransition`) | Animations defined by the framework. | Opacity and transform transitions or keyframe animations with the timing of the WinUI animation (none in the foundation; each batch records its choice). | FerroUI has no theme animations. |
| G-23 | The controls of the merged theme FerroUI has no control for (InkToolbar, MediaTransportControls, Pivot, RatingControl, PersonPicture, MapControl, TeachingTip, TwoPaneView, SwipeControl, AnnotatedScrollBar, ItemsView, ItemContainer, BreadcrumbBar, PagerControl, RadioButtons, ProgressRing, ImageIcon, ScrollView, CommandBarFlyout, RichEditBox, GridView, ListView, Hub, SemanticZoom, ContentDialog's own control, NavigationView, InfoBadge) | Styles and resources of those controls. | Their documents are waived (`scripts/winui-theme/waivers.json`, with reasons); NavigationView, InfoBar, ContentDialog, TitleBar and SelectorBar keep their resources for the FerroUI controls built from them. | No FerroUI control would use them. |

## Resources

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| RES-1 | All documents of the merged theme | Every page defines its keys; a later page replaces an earlier key. | Only the effective definitions are ported; a superseded definition (2,000 of them in GX alone) is not written. | Every key defined once, with the value a WinUI 3 application gets. |
| RES-2 | `LinearGradientBrush MappingMode="Absolute"`, `RelativeTransform` (Common_themeresources_any `ControlElevationBorderBrush`, `AccentControlElevationBorderBrush`, the text box elevation brushes) | Absolute points; a transform relative to the bounding box. | Absolute points (FerroUI's default unit); relative points as percentages; `RelativeTransform` as `Transform` with `TransformOrigin` at its centre. | FerroUI's gradients have no mapping mode property; the units of `RelativePoint` express it. |
| RES-3 | `Visibility` resources (`AppBarButtonHasFlyoutChevronVisibility`) | `Visible`/`Collapsed`. | `x:Boolean` (`True`/`False`) for `IsVisible`. | FerroUI has `IsVisible`, not `Visibility`. |
| RES-4 | `primitives:CornerRadiusFilterConverter` (CS `CornerRadius_themeresources.xaml`, `ScrollBar_themeresources.xaml`) | Filters `Top`/`Bottom`/`Left`/`Right`, or the values `TopLeftValue`/`BottomRightValue` with a `Scale`. | FerroUI's `CornerRadiusFilterConverter` with the corners of the filter; `CornerRadiusToDoubleConverter` for the values without scale; the four scaled value converters are waived. | FerroUI's double converter has no scale; the one use of the scaled ones (the ScrollBar track) is written another way (SB-8). |
| RES-5 | `primitives:CornerRadiusToThicknessConverter` (6 keys of `CornerRadius_themeresources.xaml`), `TextTrimming` `MenuFlyoutItemTextTrimming`, `Symbol` `InfoBarCloseButtonSymbol`, `DataTemplate` `TreeViewItemDataTemplate`, `ControlTemplate` `ScrollViewerScrollBarlessTemplate` | Resources of types FerroUI lacks or of templates. | Pending (listed in TRACKING.md); the batch that needs one ports it with its control. | No FerroUI type to convert to, or part of a control template. |
| RES-6 | `TextCommandBarFlyout` (`TextControlCommandBarContextFlyout`, `TextControlCommandBarSelectionFlyout`, CS `Common_themeresources.xaml`) | The context and selection flyouts of the text controls. | Waived; the text box gets a menu flyout of the same commands with batch B3 (TextBox TXB-9). | FerroUI has no command bar flyout. |
| RES-7 | Input validation templates of GX (`DefaultInputValidationErrorTemplate`, `DefaultCompactErrorIconTemplate`, `RequiredHeaderContent`) | Templates of input validation, removed from WinUI 3 before release. | Waived. | Inert in WinUI 3; FerroUI's DataValidationErrors is built from the error brushes. |
| RES-8 | `ListViewItemPresenterCheckMode`, `*RoundedChromeEnabled`, `*SelectionCheckMarkVisualEnabled`, `ThemeShadowIsUsingDropShadows`, `IsDefaultShadowEnabled` | Switches of chrome drawn in native code. | Waived. | The FerroUI templates draw that chrome with elements and have no such switch. |
| RES-9 | `controls/dev/dll/DensityStyles/Compact.xaml`: `<Style TargetType="TextBlock">` (font size 14) | The compact density sets the font size of every text block. | Pending. | As a default control theme of `TextBlock` it would override the inherited font size of the text blocks of every template; the batches decide. |

## AdornerLayer

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| ADL-1 | (no style) the system focus visual of WinUI; CS `Common_themeresources.xaml` `SystemControlFocusVisualPrimaryBrush`, `SystemControlFocusVisualSecondaryBrush` | The framework draws focus visuals. | The default focus adorner: a 2 px border in the primary brush around a 1 px border in the secondary brush, corner radius `ControlCornerRadius`. | FerroUI draws focus visuals in the adorner layer (G-14). |

## Button

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| BTN-1 | CS `Button_themeresources.xaml`, `DefaultButtonStyle`, `SubtleButtonStyle`: `Setter Target="ContentPresenter.(controls:AnimatedIcon.State)"` | Sets the state of an animated icon in the content. | Left out. | No `AnimatedIcon` (G-10). |
| BTN-2 | `DefaultButtonStyle`, `AccentButtonStyle`, `SubtleButtonStyle`: `ContentPresenter x:Name="ContentPresenter"` | The content presenter of the template. | `PART_ContentPresenter`. | The template part FerroUI's content controls host their content in. |
| BTN-3 | `ContentPresenter.BackgroundTransition` (`BrushTransition Duration="0:0:0.083"`) | Animates background changes. | `BrushTransition Property="Background"` in `ContentPresenter.Transitions`. | FerroUI's transitions are a list on the element. |
| BTN-4 | `AccentButtonStyle`, `SubtleButtonStyle` (no `BasedOn`) | Applied over the default style. | Based on `DefaultButtonStyle`; `SubtleButtonStyle`, whose template is the one of `DefaultButtonStyle` in the source, takes it from its base. | G-15. |
| BTN-5 | `FocusVisualMargin="-3"`, `UseSystemFocusVisuals` | Focus visual 3 px outside the button. | `FocusAdorner` with `Margin="-3"`. | G-14. |
| BTN-6 | (not in the source) | An accent button is a button with `Style="{StaticResource AccentButtonStyle}"`. | Also the style class `accent` (`Classes="accent"`) of the default theme: the brushes and states of `AccentButtonStyle`. | The framework's applications and samples (the ControlCatalog) mark accent buttons with the class, as the Fluent theme defines it; without it they would show as default buttons. |
| BTN-7 | GX `TextBlockButtonStyle` (target type `ButtonBase`) | A keyed style of any button, applied over its default style. | A keyed control theme of `Button` based on `DefaultButtonStyle`; it applies to Button and the classes derived from it. | ButtonBase maps to Button (section 6); G-15. |
| BTN-8 | `TextBlockButtonStyle` template: `Grid` with margin and background, `ContentPresenter x:Name="Text"` | The template. | `Panel` with margin and background; `PART_ContentPresenter` with `RecognizesAccessKey`. | FerroUI panels have a background; template part (BTN-2); G-20. |

## Calendar

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| CAL-1 | CS `CalendarView_themeresources.xaml`, `DefaultCalendarViewStyle`: one template with the header, the navigation buttons and the views | CalendarView is one control. | The theme of `Calendar` has the setters of the style and a template of `PART_Root` (a `Panel`) with `PART_CalendarItem`; the template of the source is the theme of `CalendarItem`. | FerroUI's Calendar hosts a CalendarItem, which holds the header and the views. |
| CAL-2 | `DefaultCalendarViewStyle`: the 40 setters of the item brushes, fonts and margins (`TodayBackground`, `SelectedBorderBrush`, `CalendarItemForeground`, `BlackoutStrikethroughBrush`, `DayItemFontSize`, `TodayFontWeight`, `CalendarItemBorderThickness`, ...) | Properties of CalendarView that the native chrome of the items reads; an application can set them on the calendar. | Left out; the themes of `CalendarDayButton` and `CalendarButton` take the same resources directly. | FerroUI's Calendar has no such properties; the resources keep the values of WinUI. |
| CAL-3 | `IsTabStop="False"`, `UseSystemFocusVisuals` | The calendar is not a tab stop; its items take the focus and draw the focus visual. | `IsTabStop` left out, `FocusAdorner="{x:Null}"`; the focused day draws the focus visual (CDB-6). | FerroUI's calendar takes the keyboard focus itself and moves a current day. |
| CAL-4 | `HorizontalContentAlignment`, `VerticalContentAlignment` (`Stretch`) | Align the grid of the template. | Left out. | FerroUI's Calendar has no such properties; its item stretches in the root panel. |

## CalendarButton

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| CLB-1 | CalendarViewItem (no style); `dxaml/xcp/core/core/elements/CalendarViewBaseItemChrome.cpp` | The month and year items are drawn by native chrome: an outer border with the background and the border of the state, the text, the focus visual. | A template of `Border OuterBorder`, `PART_ContentPresenter` and `Border FocusVisual`, with the state rules of the chrome as nested styles. | FerroUI has no native chrome; the elements draw the same. |
| CLB-2 | `CalendarViewBaseItemChrome::GetOuterBorderCornerRadius` (rounded chrome, `CalendarViewBaseItemRoundedChromeEnabled`) | The corner radius is half the actual size: a circle. | `CornerRadius="36.75"`, half of the 73.5 by 74 px of an item of the year view of the port (CLI-4). | A style cannot compute the radius from the size; the size is fixed by the template of the item. |
| CLB-3 | The today state of a month or year item (`CalendarViewGeneratorHost::PrepareItemContainer`); no selection | The current month or year has the today look (accent background) while `IsTodayHighlighted`. | FerroUI's `:selected` (the month or the year of the displayed date) takes the today look (`CalendarViewTodayBackground`, hover, pressed, disabled). | FerroUI's CalendarButton has no today state; the displayed month is the current one unless the application moves it. |
| CLB-4 | `DefaultCalendarViewStyle` brushes (`CalendarItemBackground`, `HoverBorderBrush`, `OutOfScopeForeground`, ...) | Read from the calendar. | The resources of the brushes; `MonthYearItemFontSize` (`CalendarViewMonthYearItemFontSize`), `MonthYearItemFontFamily` (`XamlAutoFontFamily`, G-8) and `MonthYearItemMargin` are setters of the theme. | CAL-2. |
| CLB-5 | `IsGroupLabelVisible` (`FirstOfYearDecadeLabelFontSize`, `CalendarViewFirstOfYearDecadeLabelMargin`) | A label above the first month of a year or the first year of a decade, when enabled. | Not ported. | FerroUI's calendar has no group labels; WinUI's default is off. |
| CLB-6 | System focus visual of the item (keyboard focus) | Drawn while the item has the keyboard focus. | `Border FocusVisual` (2 px `CalendarViewFocusVisualPrimaryBrush`, 1 px secondary, margin -3) shown in `:btnfocused`. | FerroUI's month buttons are not focused; the calendar marks the item with its focus. |
| CLB-7 | (not in the source) | The item reacts on press. | `ClickMode="Release"`. | FerroUI's calendar selects on release, as the Fluent theme sets it. |

## CalendarDatePicker

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| CDP-1 | CS `CalendarDatePicker_themeresources.xaml`, `DefaultCalendarDatePicker`: `TextBlock x:Name="DateText"` | Shows the date or the placeholder; the date is not edited. | `PART_TextBox` with the keyed theme `CalendarDatePickerTextBoxStyle`: a transparent text box without a border, a delete button or a focus visual, the placeholder drawn as the text block draws it, and the floating placeholder of FerroUI's `UseFloatingPlaceholder` (bound from the picker) above the text. | FerroUI's CalendarDatePicker edits the date in a text box. |
| CDP-2 | `FontIcon x:Name="CalendarGlyph"` (E787 at 12 px) in a column of 32 px; the whole control opens the flyout | A glyph. | `PART_Button` (keyed theme `CalendarDatePickerButtonStyle`, transparent, not focusable) in the column of 32 px, with a `Path` of `GlyphGeometryE787`, 12 px wide, as its content. | FerroUI's control opens the calendar with its button part; G-9. |
| CDP-3 | `DateText` `Padding="12, 0, 0, 2"` | Fixed in the template. | `Padding` setter of the control, bound to the text box. | So that FerroUI's `Padding` of the control applies. |
| CDP-4 | `DateText` `Text="{TemplateBinding PlaceholderText}"` in the foreground of the state | The placeholder is drawn in the foreground of the text. | `PlaceholderForeground` setter `CalendarDatePickerTextForeground`; the states set `Foreground` and `PlaceholderForeground` of the text box. | FerroUI's text box draws its placeholder in `PlaceholderForeground`. |
| CDP-5 | `FlyoutBase.AttachedFlyout`: `Flyout Placement="Bottom" ShouldConstrainToRootBounds="False"` with a `FlyoutPresenterStyle` (no padding, no border, `IsDefaultShadowEnabled`, `OverlayCornerRadius`) | A flyout with a shadow that may leave the window. | `Popup PART_Popup` (light dismiss, `Placement="Bottom"`) with `Calendar PART_Calendar` (`OverlayCornerRadius`); no presenter, no shadow. | FerroUI's control requires the popup and the calendar parts. |
| CDP-6 | `CalendarView` bindings `Style="{TemplateBinding CalendarViewStyle}"`, `MinDate`, `MaxDate`, `DisplayMode`, `DayOfWeekFormat`, `CalendarIdentifier`, `IsOutOfScopeEnabled`, `IsGroupLabelVisible`; setter `CalendarViewStyle` | The calendar takes the properties of the picker. | The calendar is bound to FerroUI's `FirstDayOfWeek`, `IsTodayHighlighted`, `SelectedDate`, `DisplayDate`, `DisplayDateStart`, `DisplayDateEnd`, `IsWeekNumberVisible`, `WeekNumberRule`; its theme is the default one. | The properties FerroUI's CalendarDatePicker has. |
| CDP-7 | `SelectionStates` (`Selected`: `DateText` and `CalendarGlyph` in `CalendarDatePickerTextForegroundSelected`) | A selected date is drawn in the selected foreground in every state. | Not ported. | FerroUI's control has no state for a selected date; the brush is the one of the normal state. |
| CDP-8 | `HeaderContentPresenter`, `DescriptionPresenter`, their rows, `HeaderStates` | Header and description. | Left out. | FerroUI's CalendarDatePicker has no `Header` or `Description`. |
| CDP-9 | `UseSystemFocusVisuals`, `FocusVisualMargin="-3"`, `Control.IsTemplateFocusTarget` on `Border Background` | The focus visual around the border. | `FocusAdorner` with margin -3 and `TemplatedControl.IsTemplateFocusTarget` on `Border Background`; the text box has no focus visual. | G-14; while the text box has the focus, FerroUI shows its caret, WinUI has no text box. |
| CDP-10 | `CalendarDatePickerLightDismissOverlayBackground` | The overlay of a light dismiss flyout. | Not used. | FerroUI's popups have no light dismiss overlay. |
| CDP-11 | (not in the source) the error state and `DataValidationErrors` of the Fluent theme | WinUI 3 has no input validation. | No error state and no `DataValidationErrors` wrapper. | As the text box (TXB-1); the wrapper comes with the theme of DataValidationErrors (batch B1). |

## CalendarDayButton

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| CDB-1 | CS `CalendarView_themeresources.xaml`, `{CalendarViewDayItem}`: an empty `ControlTemplate`; `CalendarViewBaseItemChrome.cpp` | The native chrome draws the day: an outer border (background, border of `CalendarItemBorderThickness`), an inner border inside it, the text, a strike-through line, the focus visual. | A template of `Border OuterBorder`, `Border InnerBorder`, `PART_ContentPresenter`, `Path BlackoutStrikethrough` and `Border FocusVisual`. | FerroUI has no native chrome; the elements draw the same. |
| CDB-2 | `GetOuterBorderCornerRadius` (rounded chrome) | Half the actual size: a circle. | `CornerRadius="20"` (the item is 40 by 40 px, `MinWidth`, `MinHeight`), inner border 19. | A style cannot compute the radius from the size. |
| CDB-3 | `DefaultCalendarViewStyle` brushes and fonts of the items | Read from the calendar. | The resources of the brushes; `DayItemFontSize` (`CalendarViewDayItemFontSize`), `DayItemFontFamily` (`XamlAutoFontFamily`, G-8) and `TodayFontWeight` are setters of the theme. | CAL-2. |
| CDB-4 | `GetItemBackgroundBrush`, `GetItemBorderBrush`, `GetTextBlockForeground`, `SetBlackoutStrikethroughBrush`, `GetItemInnerBorderBrush` (rounded chrome) | Code chooses the brush of each element by priority: disabled, today, blackout, selected, out of scope, pressed, hovered. | Nested styles on `:pointerover`, `:pressed`, `:inactive` (out of scope), `:selected`, `:blackout`, `:today`, `:disabled` and their combinations, ordered so that the last matching style is the brush the chrome chooses. | G-1. |
| CDB-5 | `SetBlackoutStrikethroughSize`, `s_strikethroughFontSizeMultiplier` (1.25) | A diagonal line over 1.25 times the font size, centred, of 1 px; created only for a blackout day. | A `Path` of 18 by 18 px (1.25 x 14 px), centred, 1 px, shown in `:blackout`. | Sized for the default day font size; a style cannot read the font size into geometry. |
| CDB-6 | `FocusVisualMargin="-3"`, `FocusVisualPrimaryBrush` (`CalendarViewFocusVisualPrimaryBrush`, 2 px), `FocusVisualSecondaryBrush` (1 px), `UseSystemFocusVisuals` | The system draws a circular focus visual while the day has the keyboard focus. | `Border FocusVisual` (margin -3, corner radius 23) shown in `:dayfocused`: whenever the calendar has the focus on that day, also after a pointer press. | FerroUI's day buttons are not focused; the calendar marks the current day. |
| CDB-7 | `IsGroupLabelVisible`, `FirstOfMonthLabelFontSize`, `CalendarViewFirstOfMonthLabelMargin` | The label of the month on its first day, when enabled. | Not ported. | FerroUI's calendar has no group labels; WinUI's default is off. |
| CDB-8 | `DayItemMargin`, `HorizontalDayItemAlignment`, `VerticalDayItemAlignment` of CalendarView | The margin and alignment of the text of the day. | `Padding` of the content presenter (`CalendarViewDayItemMargin`) and the content alignments of the theme (centre). | FerroUI's Calendar has no such properties. |
| CDB-9 | (not in the source) | The item reacts on press. | `ClickMode="Release"`. | FerroUI's calendar selects a range by dragging over the days; as the Fluent theme. |

## CalendarItem

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| CLI-1 | CS `CalendarView_themeresources.xaml`, template of `DefaultCalendarViewStyle`: `HeaderButton`, `PreviousButton`, `NextButton` | Named elements the control finds. | `PART_HeaderButton`, `PART_PreviousButton`, `PART_NextButton`. | FerroUI's template parts. |
| CLI-2 | `MonthViewScrollViewer`, `YearViewScrollViewer`, `DecadeViewScrollViewer` with `CalendarPanel`s and the template-local `ScrollViewerStyle` (`ScrollViewerScrollBarlessTemplate`, vertical snap points) | Virtualized panels in scroll viewers: the views scroll through the months, years and decades. | The grids `PART_MonthView` (the day names in its first row, six weeks below) and `PART_YearView`, which FerroUI's item fills; the views page with the previous and next buttons and do not scroll; `ScrollViewerStyle` is not ported. | FerroUI's CalendarItem fills grids and has no scrolling views. |
| CLI-3 | `Grid x:Name="WeekDayNames"` (margin 2,2,2,0) above `MonthViewScrollViewer` (margin 2) | The day names and the days in two elements, 2 px apart. | The day names in the first row of `PART_MonthView` (margin 2), their text blocks with a bottom margin of 3 (`CalendarViewWeekDayMargin` and the gap). | FerroUI's item puts the day names into the grid of the days. |
| CLI-4 | The year and decade views: 16 items in 4 rows of 4 (the months of the year and of the next, the years of the decade and around it) | 16 items. | `PART_YearView` has 4 rows and 4 columns, of which FerroUI's item fills 3 rows (12 months; the decade with the year before and after); the fourth row stays empty. | FerroUI's item has 12 items; the 4 rows keep the size of WinUI's items (73.5 by 74 px). |
| CLI-5 | `MinWidth="{Binding TemplateSettings.MinViewWidth}"`, the height of the views from the month view | Computed by the control. | The views have `MinHeight="300"` (2 + 42 + 2 + 6 x 42 + 2, the month view of the source); the width follows the day items. | FerroUI's item has no template settings; the year view needs the height of the month view. |
| CLI-6 | `HeaderButtonStates` (`ViewChanging`: the header fades in over 167 ms) | Animates the header when the view changes. | Not ported. | FerroUI's item has no state for a view change. |
| CLI-7 | The transitions of `DisplayModeStates` (scales 0.84 and 1.29, opacities over 150 and 500 ms with `ControlFastOutSlowInKeySpline`), `BackgroundLayer`, `MonthViewTransform`, `YearViewTransform`, `DecadeViewTransform` | Zooms between the month, year and decade views. | Not ported; the views switch by their visibility. | FerroUI's item sets the visibility of the grids in code, without a state to animate from. |
| CLI-8 | `Grid.Clip` of the views (`TemplateSettings.ClipRect`) | Clips the zoom animations. | `ClipToBounds="True"` on the views. | No template settings. |
| CLI-9 | `NavigationButtonStyle` content `&#xEDDB;`, `&#xEDDC;` in `SymbolThemeFontFamily` at `CalendarViewNavigationButtonFontSize` (8) | Glyphs of the symbol font (up for previous, down for next: the views scroll vertically). | Paths of `GlyphGeometryEDDB` and `GlyphGeometryEDDC`, 8 px wide in a box of 8 px, filled with the foreground of the content presenter. | G-9; the directions of the source are kept although FerroUI's views page. |
| CLI-10 | Template-local `WeekDayNameStyle`, `HeaderNavigationButtonStyle`, `NavigationButtonStyle` (`ContentPresenter x:Name="Text"`) | Styles of the template. | Keyed themes `CalendarItemWeekDayNameStyle` (based on `CaptionTextBlockStyle`), `CalendarItemHeaderNavigationButtonStyle`, `CalendarItemNavigationButtonStyle`; `Text` is `PART_ContentPresenter`. The button themes are not based on `DefaultButtonStyle`: they write the setters of the default style that take effect (font family, border thickness). | G-18; the state styles of `DefaultButtonStyle` would set brushes of `PART_ContentPresenter` that the source's states leave alone (the pressed border, the disabled background). |
| CLI-11 | `Disabled` state: the foreground of `WeekDay1`..`WeekDay7` | Seven animations. | `:calendardisabled` sets `TextElement.Foreground` of `PART_MonthView`, which the day names inherit (also the normal foreground, `CalendarViewCalendarItemForeground`). | The day names are created by the item from its `DayTitleTemplate`. |
| CLI-12 | (not in the source) | WinUI's calendar has no week numbers. | `PART_ElementWeekNumberLabels`: a column of the week numbers at `CaptionTextBlockFontSize` in `TextFillColorSecondaryBrush`, shown while `IsWeekNumberVisible`. | A template part of FerroUI's item. |
| CLI-13 | (not in the source) | No header background. | `HeaderBackground` of the item (null by default) is the background of the header grid. | A property of FerroUI's item. |
| CLI-14 | `HeaderNavigationButtonStyle`, `NavigationButtonStyle`: `IsTabStop`, `UseSystemFocusVisuals`, `FocusVisualMargin` -2 | The buttons are tab stops with focus visuals. | The focus adorners (margin -2) are kept; FerroUI's item makes the buttons not focusable. | FerroUI's calendar handles the keyboard itself. |

## CheckBox

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| CHK-1 | CS `CheckBox_themeresources.xaml`, `DefaultCheckBoxStyle`: `Grid x:Name="RootGrid"` with background, border and corner radius | The root panel draws the background and border the states change. | `Border x:Name="RootGrid"` around a `Grid`. | G-17. |
| CHK-2 | `controls:AnimatedIcon x:Name="CheckGlyph"` with `AnimatedAcceptVisualSource` and the fallback `FontIconSource` (`CheckBoxCheckedGlyph` E73E, `CheckBoxIndeterminateGlyph` E9AE, `CheckBoxGlyphSize` 12) | An animated check mark; the fallback glyphs of the symbol font. | A `Path` with `GlyphGeometryE73E` (10 px wide) when checked and `GlyphGeometryE9AE` (8 px wide, margin 0) when indeterminate; hidden (opacity 0) when unchecked. | G-9, G-10. The widths are the estimated ink of the glyphs at 12 px. |
| CHK-3 | The `AnimatedIcon.State` setters of `CombinedStates` (`NormalOff`, `PointerOverOn`, ...) | Select the frame of the animation. | The opacity and data of the path per check state. | G-10. |
| CHK-4 | `ContentPresenter x:Name="ContentPresenter"` | The content presenter. | `PART_ContentPresenter`. | Template part (BTN-2). |
| CHK-5 | `FocusVisualMargin="{StaticResource CheckBoxFocusVisualMargin}"` (-7,-3,-7,-3) | Focus visual around box and label. | `FocusAdorner` with that margin. | G-14. |

## DataValidationErrors

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| DVE-1 | GX `DefaultInputValidationErrorTemplate`, `DefaultCompactErrorIconTemplate` (waived, RES-7) | Input validation was removed from WinUI 3; an application shows the error of a field itself, with the critical fill of the palette. | The default theme shows the errors of the owner under its content, 4 px below it, in `SystemFillColorCriticalBrush`, wrapped; a control that wraps its template in `DataValidationErrors` draws its border in `SystemFillColorCriticalBrush` under `:error` (TXB-12). | FerroUI's controls report validation errors through `DataValidationErrors`; the values are those of an error of a field in a WinUI 3 application. |
| DVE-2 | (no counterpart) the keyed theme `TooltipDataValidationErrors` of FerroUI | No compact error presentation. | The errors in the tool tip of an error icon at the right of the content: the error icon of InfoBar (`controls/dev/InfoBar/InfoBar.xaml`), `InfoBarIconBackgroundGlyph` (F136) in `InfoBarErrorSeverityIconBackground` under `InfoBarErrorIconGlyph` (F13D) in `InfoBarErrorSeverityIconForeground`, at `InfoBarIconFontSize` (16), 8 px from the content. | The icon WinUI shows for an error; G-9 for the glyphs, whose cross is estimated at 6 px. |
## DatePicker

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| DP-1 | CS `DatePicker_themeresources.xaml`, `DefaultDatePickerStyle`: `FlyoutButton`, `FlyoutButtonContentGrid`, `DayTextBlock`, `MonthTextBlock`, `YearTextBlock`, `FirstPickerSpacing`, `SecondPickerSpacing` | Named elements the control finds. | `PART_FlyoutButton`, `PART_ButtonContentGrid`, `PART_DayTextBlock`, `PART_MonthTextBlock`, `PART_YearTextBlock`, `PART_FirstSpacer`, `PART_SecondSpacer`. | FerroUI's template parts. |
| DP-2 | `DatePickerFlyout` (created by `DatePicker_Partial.cpp`) | A flyout the control opens over the button. | `Popup PART_Popup` (light dismiss, `WindowManagerAddShadowHint="False"`) with `DatePickerPresenter PART_PickerPresenter` in the template; the control places it over the button. | FerroUI's DatePicker requires the popup and the presenter parts. |
| DP-3 | `Text="Day"`, `"Month"`, `"Year"` of the template; `TEXT_DATEPICKER_DAY_PLACEHOLDER`, ... (`dxaml/xcp/dxaml/dllsrv/winrt/Microsoft.UI.Xaml.Common.rc`) | The control replaces the texts with the localized placeholders ("day", "month", "year" in English). | The English placeholders are the texts of the template; FerroUI's control restores them when it has no date. | FerroUI's control sets no placeholder text; the strings of WinUI are not localized here. |
| DP-4 | Template-local `DatePickerFlyoutButtonStyle` (`Grid` > `ContentPresenter x:Name="ContentPresenter"`, `ElementSoundMode`) | The style of the flyout button. | Keyed theme `DatePickerFlyoutButtonStyle`, not based on `DefaultButtonStyle`; `Panel` > `PART_ContentPresenter`; `ElementSoundMode` left out. | G-18; FerroUI has no element sounds. |
| DP-5 | `IsTabStop="False"` of the control; `UseSystemFocusVisuals`, `FocusVisualMargin="-3"` given to the flyout button | The flyout button takes the focus and draws the focus visual. | The `FocusAdorner` of the control (margin -3) is drawn around the flyout button (`TemplatedControl.IsTemplateFocusTarget`); `IsTabStop` is left out. | FerroUI's DatePicker takes the focus itself. |
| DP-6 | `HeaderContentPresenter` and its row, `Orientation`, the first `FocusVisualMargin="0"` setter (replaced by `-3` in the same style) | Header and orientation. | Left out. | FerroUI's DatePicker has no `Header` or `Orientation`. |
| DP-7 | (not in the source) | The text blocks are top aligned in the button. | The text blocks follow `VerticalContentAlignment` of the control. | A property of FerroUI's DatePicker, as the Fluent theme. |
| DP-8 | `DefaultDatePickerFlyoutPresenterStyle`: the `LoopingSelector`s of the day, month and year that `DatePickerFlyoutPresenter_Partial.cpp` puts into `PickerHostGrid` | Looping lists with up and down buttons. | `PickerHostGrid` is `PART_PickerContainer` with the hosts `PART_DayHost`, `PART_MonthHost`, `PART_YearHost`: the template of `DefaultLoopingSelectorStyle` written out (DTP-5) around the `DateTimePickerPanel`s `PART_DaySelector`, `PART_MonthSelector`, `PART_YearSelector`. | FerroUI's presenter requires the hosts and the panels. |
| DP-9 | The columns of `PickerHostGrid` (78*, 132*, 78*, set in code) | The month column is 132*. | FerroUI's presenter and control set the month column to 138*. | Set by the code of FerroUI's controls. |
| DP-10 | `HighlightRect` (a `Grid` with background and corner radius, above the items, `Canvas.ZIndex` 9999) and three `primitives:MonochromaticOverlayPresenter`s (10000) | The highlight covers the selected items, which the overlay presenters draw again in `DatePickerFlyoutPresenterHighlightForegroundColor`. | `Border HighlightRect` under the items (`ZIndex` -1); the selected items are drawn in `TextOnAccentAAFillColorPrimaryBrush` (DTP-8). | FerroUI has no monochromatic overlay presenter. |
| DP-11 | `AcceptButton`, `DismissButton` with `&#xE8FB;`, `&#xE711;` in `SymbolThemeFontFamily` at 16 px | Glyphs of the symbol font. | `PART_AcceptButton`, `PART_DismissButton` with paths of `GlyphGeometryE8FB` (14 px wide) and `GlyphGeometryE711` (12 px wide). | G-9. |
| DP-12 | The `ThemeShadow` of the flyout | A shadow under the flyout. | No shadow. | A FerroUI popup has no shadow of its own (`WindowManagerAddShadowHint` off, as in the Fluent theme). |
| DP-13 | `DatePickerFlyoutPresenter` `AutomationProperties.AutomationId`, `DatePickerLightDismissOverlayBackground` | Automation id; the light dismiss overlay. | Left out. | G-19; FerroUI's popups have no light dismiss overlay. |
| DP-14 | `MinWidth="{StaticResource DatePickerThemeMinWidth}"`, `MaxWidth="{StaticResource DatePickerThemeMaxWidth}"` on the flyout button | The sizes of the button, which an application cannot change. | Setters `MinWidth` and `MaxWidth` of the control (the same resources), bound to the button. | FerroUI's DatePicker honours its own `MinWidth` and `MaxWidth`, as the Fluent theme. |
| DP-15 | (not in the source) the error state and `DataValidationErrors` of the Fluent theme | WinUI 3 has no input validation. | No error state and no `DataValidationErrors` wrapper. | As CDP-11. |

## DateTimePickerShared

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| DTP-1 | CS `DateTimePickerFlyout_themeresources.xaml`, `DateTimePickerFlyoutButtonStyle`, `DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle` (target type `ButtonBase`) | Keyed styles applied over the default style of a Button or a RepeatButton. | Keyed control themes of `Button` (they apply to RepeatButtons too), not based on `DefaultButtonStyle`; the setters of the default style that take effect (the foreground `ButtonForeground`, the font) are written. | ButtonBase maps to Button; the state styles of `DefaultButtonStyle` would set the background, the border and the foreground of `PART_ContentPresenter` in states the source does not style (pressed border, disabled). |
| DTP-2 | `DateTimePickerFlyoutButtonStyle` template: `Grid` with the background, `ContentPresenter x:Name="ContentPresenter"` | The template. | `Panel` with the background, `PART_ContentPresenter`. | Panels of FerroUI have a background; template part (BTN-2). |
| DTP-3 | `DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle` template: `TextBlock x:Name="Content"` with `Text="{TemplateBinding Content}"` (the glyph as text) | The content is a glyph string in the symbol font of the button. | `PART_ContentPresenter`; the glyph is a `Path` content of the keyed themes `DateTimePickerLoopingSelectorUpButtonStyle` and `DateTimePickerLoopingSelectorDownButtonStyle` (G-9). | Glyph geometries instead of the symbol font. |
| DTP-4 | The `Pressed` state: `ScaleTransform` to `LoopingSelectorUpDownButtonScalePressed` (0.875) at 16 ms, `RepeatBehavior="Forever"` | Shrinks the glyph while pressed. | `RenderTransform="scale(0.875)"` in `:pressed`. | G-2 (the value is reached after one frame), as SB-5. |
| DTP-5 | `DefaultLoopingSelectorStyle`, `{LoopingSelector}`: `ScrollViewer` (`ScrollViewerScrollBarlessTemplate`, mandatory snap points), `UpButton`, `DownButton` (`&#xEDDB;`, `&#xEDDC;` at 8 px, collapsed, shown in `PointerOver`), `ShouldLoop`, `ItemTemplate` | A templated control of a looping list. | Written into the host panels of the presenters (DP-8, TP-8): a `ScrollViewer` with hidden bars around the `DateTimePickerPanel`, the RepeatButtons with the keyed themes `DateTimePickerLoopingSelectorUpButtonStyle` and `DateTimePickerLoopingSelectorDownButtonStyle`, shown by a `:pointerover` style of the host; `ShouldLoop` is set on the panel; the snapping is the panel's. | FerroUI's DateTimePickerPanel is a panel, not a templated control. |
| DTP-6 | `{LoopingSelectorItem}` | The implicit style of the items. | The keyed theme `DateTimePickerLoopingSelectorItemStyle` of `ListBoxItem`, set on the items by a style of `PART_PickerContainer` of each presenter. | The items of FerroUI's panel are ListBoxItems, whose default theme is the one of a list box. |
| DTP-7 | `LoopingSelectorItem` template: `Grid x:Name="Root"` with a corner radius, `ContentScaleTransform` | The root draws the background of the state. | `Border x:Name="Root"`; the scale transform (not animated in the source) is left out. | G-17. |
| DTP-8 | The `Selected` state (`LoopingSelectorItemForegroundSelected`, `LoopingSelectorItemBackgroundSelected`) under the highlight and the overlay presenters of the flyouts | The selected item is covered by the highlight and drawn again in `TextOnAccentAAFillColorPrimary` (`*FlyoutPresenterHighlightForegroundColor`). | `:selected`: no background of the item, the foreground `TextOnAccentAAFillColorPrimaryBrush`, also under the pointer; in the high contrast variant its colour is the window text, where the replacement colour of the source is transparent. | The highlight is under the items (DP-10). |
| DTP-9 | The padding and the alignment of the items, set by the code of the presenters on each `LoopingSelector` (`DatePickerFlyoutPresenterItemPadding`, `DatePickerFlyoutPresenterMonthPadding` with the month left aligned, `TimePickerFlyoutPresenterItemPadding`) | Per column. | Setters of the classes of the items (`MonthItem`, `HourItem`, `MinuteItem`, `SecondItem`, `TimePeriodItem`; the day and year items take the default). | FerroUI's panel gives its items a class of their column. |
| DTP-10 | `DefaultLoopingSelectorStyle`: `CornerRadius`, `UseSystemFocusVisuals` | The looping list draws a focus visual when it has the keyboard focus. | No focus visual on the panel. | FerroUI's DateTimePickerPanel is not templated and has no focus adorner of the theme. |

## EmbeddableControlRoot

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| ECR-1 | (no counterpart) | WinUI 3 has no root of content hosted in another UI with a style. | The template FerroUI's control requires (from the Fluent theme), with `SolidBackgroundFillColorBaseBrush`, `TextFillColorPrimaryBrush`, `ContentControlThemeFontFamily`, `ControlContentThemeFontSize`. | Built from the resources of a WinUI window. |

## GridSplitter

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| GS-1 | (no counterpart in the merged theme) | WinUI 3 has no grid splitter (the splitters of the Windows Community Toolkit are not part of WinUI). | A bar of `ControlStrongFillColorDefaultBrush`, 4 px thick (`MinWidth`, `MinHeight`), `ControlStrongFillColorDisabledBrush` when disabled, without pointer-over or pressed states; the preview of a drag a rectangle of `AccentFillColorDefaultBrush`. | Built from the fills WinUI gives the strong parts of its controls (the thumb of a scroll bar, the knob of a toggle switch) and from the accent. |
| GS-2 | (no counterpart) | No focus. | Focusable, with the default focus adorner of the theme. | FerroUI's splitter is moved with the keyboard when it has the focus. |

## GroupBox

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| GB-1 | `controls/dev/Expander/Expander.xaml` `DefaultExpanderStyle` in its expanded state | WinUI has no group box. | An expanded Expander without its chevron: the header bar (`ToggleButtonGrid` of `ExpanderHeaderDownStyle`, a Border) with `PART_HeaderPresenter` over the content border (`ExpanderContent`) with `PART_ContentPresenter`, their corners the corner radius through `TopCornerRadiusFilterConverter` and `BottomCornerRadiusFilterConverter`. | The WinUI control that shows a header over a card of content. |
| GB-2 | `controls/dev/Expander/Expander_themeresources.xaml` `ExpanderHeaderDownStyle`: `CommonStates`, `ExpandCollapseChevron` (AnimatedIcon) | The header is a toggle button with pointer, pressed and checked states and a chevron. | Only `Disabled` (`ExpanderHeaderDisabledForeground`, `ExpanderHeaderDisabledBorderBrush`); no chevron, no pointer or pressed state. | The header of a group box toggles nothing. |
| GB-3 | `DefaultExpanderStyle`: `HorizontalAlignment` Left, `VerticalAlignment` Center, `MinWidth` `FlyoutThemeMinWidth`, `IsTabStop` False | Places the expander and keeps it out of the tab order. | Not set: the group box stretches in its container. | A group box fills its container, as in FerroUI's other themes. |
| GB-4 | `ExpanderContent BorderThickness="{StaticResource ExpanderContentDownBorderThickness}"` | The content border has a fixed thickness. | `{TemplateBinding BorderThickness}`, which the theme sets to the same resource. | An application can change the border of the control. |

## HeaderedContentControl

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| HCC-1 | CS `TextBox_themeresources.xaml` `DefaultTextBoxStyle`: `HeaderContentPresenter` | WinUI has no headered content control; its controls with a `Header` show it above the control. | That presenter above the content: `TextControlHeaderForeground`, `FontWeight` Normal, the margin `TextBoxTopHeaderMargin`, wrapping, aligned to the top, `TextControlHeaderForegroundDisabled` when disabled. | The header of WinUI's controls. |
| HCC-2 | `HeaderContentPresenter Visibility="Collapsed" x:DeferLoadStrategy="Lazy"`, shown by the code of the control | Collapsed without a header. | `IsVisible` bound to `Header` through `ObjectConverters.IsNotNull`. | The same condition in markup. |
| HCC-3 | (not in the source) | No border, background or padding around a header and its control. | A border of the `Background`, `BorderBrush`, `BorderThickness`, `CornerRadius` and `Padding` of the control (none set by the theme) around both; the header takes the content alignments of the control. | The contract of FerroUI's control in its other themes. |

## HyperlinkButton

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| HLB-1 | `dxaml/xcp/dxaml/lib/HyperlinkButton_Partial.cpp` (`SetCursor(MouseCursorHand)`) | The control sets the hand cursor in code. | `Cursor="Hand"` setter in `DefaultHyperlinkButtonStyle`. | FerroUI's hyperlink button does not set a cursor itself. |
| HLB-2 | CS `HyperlinkButton_themeresources.xaml` (no visited state) | A visited hyperlink button looks the same. | No style for FerroUI's `:visited`. | Faithful; noted because the Fluent theme styles `:visited`. |
| HLB-3 | `ContentPresenter x:Name="ContentPresenter"`, `BackgroundTransition`, `FocusVisualMargin="-3"` | As Button. | As BTN-2, BTN-3, BTN-5. | As Button. |

## ItemsControl

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| IC-1 | The built-in template of ItemsControl (no style in the merged theme) | An ItemsPresenter of the items in the `ItemsPanel`; no background or border. | `PART_ItemsPresenter` with the `ItemsPanel` of the control, in a border of its `Background`, `BorderBrush`, `BorderThickness`, `CornerRadius` and `Padding`. | FerroUI's ItemsControl draws them in its other themes; the theme sets none of them, so the control looks the same. |

## Label

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| LBL-1 | GX `{ContentControl}` | WinUI has no Label; a ContentControl aligns its content to the left and the top. | The implicit style of ContentControl is the default theme of Label (`HorizontalContentAlignment` Left, `VerticalContentAlignment` Top), and the document claims `{ContentControl}`. | FerroUI's ContentControl has a built-in template; Label is the content control the theme styles. |
| LBL-2 | GX `{ContentControl}`: `ContentPresenter Margin="{TemplateBinding Padding}"` with the content alignments as its alignments | The presenter at the padding; no background or border. | `PART_ContentPresenter` with the `Padding`, the content alignments, `Background`, `BackgroundSizing`, `BorderBrush`, `BorderThickness` and `CornerRadius` of the control. | The template of FerroUI's ContentControl: the same layout, and the background a FerroUI application may set. |
| LBL-3 | (not in the source) | A label is a TextBlock. | `RecognizesAccessKey="True"`. | FerroUI's label focuses its target by its access key (G-20). |

## OverlayPopupHost

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| OPH-1 | (no counterpart) | The popup host of WinUI is part of the framework. | The template FerroUI's control requires, with the text resources of a WinUI window. | Built from WinUI resources. |

## PathIcon

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| PI-1 | `dxaml/xcp/core/core/elements/icon.cpp` `CPathIcon::ApplyTemplate` | The control creates a Path in code: alignments Stretch, `Fill` linked to `Foreground`, `Data` linked to `Data`. | The same Path as the template, inside a Border of the `Background` of the control. | FerroUI's PathIcon is a templated control. |
| PI-2 | (no size) IconElement | No default size; the Path keeps `Stretch` None, so the geometry is drawn in its own coordinates. | The same: no `Width` or `Height`, the geometry not scaled. | Faithful; recorded because FerroUI's other themes size a PathIcon to `IconElementThemeWidth`/`IconElementThemeHeight` (20) and scale the geometry uniformly, which this theme does not. |

## PopupRoot

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| PPR-1 | (no counterpart) | The popup host of WinUI is part of the framework. | The template FerroUI's control requires, with the text resources of a WinUI window; the presenter of the popup draws the background. | Built from WinUI resources. |

## ProgressBar

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| PB-1 | `controls/dev/ProgressBar/ProgressBar.xaml` `{ProgressBar}`; CS `Deprecated_themeresources.xaml` `DefaultProgressBarStyle` (no setters) | The implicit style is the default style; the keyed style is empty and applied over it. | The keyed theme `DefaultProgressBarStyle` holds the implicit style; `{x:Type ProgressBar}` is based on it. | Keeps the key of WinUI with the look an application gets from it (G-15). |
| PB-2 | `ProgressBarTrack`, `DeterminateProgressBarIndicator`, `IndeterminateProgressBarIndicator`, `IndeterminateProgressBarIndicator2` (Rectangle) | Rectangles with `RadiusX`/`RadiusY` from the corner radius converters. | Borders with corner radii (`ProgressBarTrackCornerRadius` for the track, the corner radius of the control for the indicators); `DeterminateProgressBarIndicator` is `PART_Indicator`. | FerroUI's ProgressBar requires `PART_Indicator` as a Border and sets its width. |
| PB-3 | Inner `Border Clip="{Binding TemplateSettings.ClipRect}"` | Clips to the rectangle inside the padding that the control computes. | `ClipToBounds` of that border. | FerroUI's template settings have no clip rectangle; the bounds of the border are that area. |
| PB-4 | `controls/dev/ProgressBar/ProgressBar.cpp` (`UpdateWidthBasedTemplateSettings`, `SetProgressBarIndicatorWidth`) | `ContainerAnimationStartPosition` is -1.0 times 40 % of the width; the indeterminate widths are 40 % and 60 % of the width inside the padding and border. | FerroUI's control computes -1.8 times 40 % of the width, and the widths of the whole track. | The template settings are computed by FerroUI's control, not by the template. |
| PB-5 | `CommonStates` `Indeterminate` | Shows the indeterminate indicators, hides the track, runs the storyboard forever; the code sets the width of the determinate indicator to 0. | The nested style of `:indeterminate`: the opacities, the keyframe animations of the translation with the key times and the spline of the source; `PART_Indicator` hidden by its opacity. | G-2, section 4.5; FerroUI's control keeps the width of `PART_Indicator`. |
| PB-6 | `Error`, `Paused`, `UpdatingError`, `IndeterminateError`, `IndeterminatePaused`, `Updating` | The error and paused colours (`ProgressBarErrorForegroundColor`, `ProgressBarPausedForegroundColor`) and the moves of the indicators between those states. | Left out. | FerroUI's ProgressBar has no `ShowError` and no `ShowPaused`. |
| PB-7 | `VisualStateGroup.Transitions`: `RepositionThemeAnimation` of the indicator when the value changes, `FadeInThemeAnimation` from `Indeterminate` to `Determinate`, the colour animations from `Error` and `Paused` | Animates the changes of the bar. | Not ported: the indicator takes its new width at once. | The timing of the theme animations comes from the system (G-22). |
| PB-8 | (no counterpart) `Orientation` of FerroUI's ProgressBar | WinUI's progress bar is horizontal. | `:vertical`: the horizontal template turned (the track 1 px wide and centred, `PART_Indicator` from the bottom, the indeterminate indicators along the height with the same animations of the vertical translation), `MinWidth` `ProgressBarMinHeight`, the bar centred horizontally and stretched vertically. | FerroUI's ProgressBar has an orientation. |
| PB-9 | (no counterpart) `ShowProgressText` of FerroUI's ProgressBar | No text. | The text of `ProgressTextFormat` 8 px after the bar (under a vertical bar), in `TextFillColorPrimaryBrush`. | FerroUI's contract; the bar of WinUI is too thin to hold a text. |
| PB-10 | `ProgressBarTrack Width="{TemplateBinding Width}"` | The track is as wide as the `Width` of the control. | The track stretches. | The same width inside the bar, also without a `Width`. |

## RadioButton

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| RDB-1 | CS `RadioButton_themeresources.xaml`, `DefaultRadioButtonStyle`: `Grid x:Name="RootGrid"` | The root panel draws background and border. | `Border x:Name="RootGrid"` around a `Grid`. | G-17. |
| RDB-2 | `CheckGlyph` `Width`/`Height` `SplineDoubleKeyFrame` to 14 (PointerOver, 250 ms), 10 (Pressed, 250 ms), 14 (Disabled, 167 ms) | Animates the inner dot. | Setters of the sizes and one `DoubleTransition` of 250 ms with spline 0,0,0,1 on the ellipse (also for Disabled, where WinUI uses 167 ms); the return to 12 animates. | G-3; a transition has one duration per property. |
| RDB-3 | `PressedCheckGlyph` opacity 1 and size 10 (167 ms) in Pressed | Shows the pressed dot of an unchecked button. | Setters and a 167 ms transition of its size. | G-3. |
| RDB-4 | `CommonStates` and `CheckStates` both set `CheckGlyph.Stroke` and `PressedCheckGlyph.Opacity` | The storyboard begun last wins. | The `:checked` style, written after the pointer states, wins. | G-1; deterministic order. |
| RDB-5 | `controls/dev/RadioButtons/RadioButton.xaml` (the same implicit style) | Merged before the theme resources; identical. | One implicit control theme. | Same item of the merged theme. |
| RDB-6 | `ContentPresenter x:Name="ContentPresenter"`, `FocusVisualMargin="-7,-3,-7,-3"` | As CheckBox. | As CHK-4, CHK-5. | As CheckBox. |

## RepeatButton

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| RPB-1 | CS `RepeatButton_themeresources.xaml`, `DefaultRepeatButtonStyle` | As Button. | As BTN-2, BTN-3, BTN-5. | As Button. |

## ScrollBar

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| SB-1 | CS `ScrollBar_themeresources.xaml`, `DefaultScrollBarStyle`: `HorizontalRoot` and `VerticalRoot` in one template, shown by the control by orientation | One template with both roots. | One template per orientation, in the nested styles `^:vertical` and `^:horizontal`. | FerroUI's parts must exist once per template. |
| SB-2 | `VerticalSmallDecrease`, `VerticalLargeDecrease`, `VerticalThumb`, `VerticalLargeIncrease`, `VerticalSmallIncrease` in grid rows (and the horizontal ones) | The control sizes the large buttons and the thumb. | `PART_LineUpButton`, a `Track` with `PART_PageUpButton` (decrease button), the thumb and `PART_PageDownButton` (increase button), `PART_LineDownButton`; the vertical track is reversed. | FerroUI's scroll bar arranges with a `Track` and requires these parts. |
| SB-3 | `Grid.Resources`: `RepeatButtonTemplate`, `HorizontalIncrementTemplate`, `HorizontalDecrementTemplate`, `VerticalIncrementTemplate`, `VerticalDecrementTemplate`, `VerticalThumbTemplate`, `HorizontalThumbTemplate` | Template-local control templates. | Keyed control themes `ScrollBar<Name>`. | G-18. |
| SB-4 | `FontIcon x:Name="Arrow"` with glyphs EDD9, EDDA, EDDB, EDDC at `ScrollBarButtonArrowIconFontSize` (8) | Arrow glyphs of the symbol font. | Paths of the glyph geometries, 8 px along their width. | G-9. |
| SB-5 | Pressed arrows: `DoubleAnimationUsingKeyFrames` of the scale to `ScrollBarButtonArrowScalePressed` (0.875) at 16 ms, `RepeatBehavior="Forever"` | Shrinks the arrow while pressed. | `RenderTransform="scale(0.875)"` in `:pressed`. | G-2 (the value is reached after one frame). |
| SB-6 | `ConsciousStates` (`Expanded`, `Collapsed`, `ExpandedWithoutAnimation`, `CollapsedWithoutAnimation`) with begin times `ScrollBarExpandBeginTime`/`ScrollBarContractBeginTime` | Expands the bar after a delay. | `^[IsExpanded=true]` (FerroUI's scroll bar sets `IsExpanded` after its own delay); opacities over 83 ms and the thumb over 167 ms as transitions. | G-3; FerroUI's control owns the expansion. |
| SB-7 | `ScrollingIndicatorStates` (`NoIndicator`, `TouchIndicator`, `MouseIndicator`) and `HorizontalPanningRoot`/`VerticalPanningRoot` | Hides the bar when not scrolling; a thin panning indicator for touch. | Left out: the bar is always in the `MouseIndicator` state (thumb shown, hit testing on). | FerroUI's scroll bar has no indicator modes. |
| SB-8 | `HorizontalTrackRect`/`VerticalTrackRect` (`Rectangle` with `RadiusX`/`RadiusY` from `TopLeftCornerRadiusDoubleValueConverter2x`/`BottomRightCornerRadiusDoubleValueConverter2x`) | Rounded track rectangles. | `Border`s with the corner radius scaled by `ScrollBarTrackCornerRadiusConverter` (a `CornerRadiusFilterConverter` with `Scale="2"`); fill and stroke are background and border brush. | RES-4. |
| SB-9 | `TranslateTransform X="{StaticResource ScrollBarThumbOffset}"` of the thumb, animated to 0 when expanded | Offsets the collapsed thumb. | `RenderTransform="translateX(2px)"` (or `translateY`), `none` when expanded, with a transform transition. | G-3, G-21 (the value is written). |
| SB-10 | `Grid x:Name="Root"` with background and border set by the states | The root panel. | `Border x:Name="Root"`. | G-17. |
| SB-11 | Thumb templates, `Disabled`: fill after `ScrollBarColorChangeDuration`, opacity to 0 over `ScrollBarOpacityChangeDuration` | Fades the disabled thumb. | Fill at once, opacity with an 83 ms transition. | G-3. |

## ScrollViewer

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| SCV-1 | CS `ScrollViewer_themeresources.xaml`, `DefaultScrollViewerStyle`: `ScrollContentPresenter`, `VerticalScrollBar`, `HorizontalScrollBar` | Template parts. | `PART_ContentPresenter`, `PART_VerticalScrollBar`, `PART_HorizontalScrollBar`. | FerroUI's parts. |
| SCV-2 | Bindings `Maximum`, `Value`, `ViewportSize` of the bars; `Grid Visibility="{TemplateBinding Computed*ScrollBarVisibility}" Padding="{ThemeResource ScrollViewerScrollBarMargin}"` around each bar | The template binds the bars. | No bindings (FerroUI's scroll viewer sets the bars in code); the padding is the `Margin` of the bar. | FerroUI's contract. |
| SCV-3 | `ScrollingIndicatorStates` with the `IndicatorMode` of the bars and its delays | Hides the bars when idle. | Left out. | FerroUI's bars have no indicator modes (SB-7). |
| SCV-4 | `ScrollBarSeparatorStates` with begin times | Shows the corner separator when the bars expand. | `^[IsExpanded=true]` sets its opacity, with a 100 ms transition, no delays. | G-3. |
| SCV-5 | `VerticalScrollBarVisibility="Visible"` | Visible bars that hide until the pointer moves. | Not set (FerroUI's default `Auto`). | Without the indicator modes a visible bar would show on every scroll viewer. |
| SCV-6 | `HorizontalScrollMode`, `VerticalScrollMode`, `IsHorizontalRailEnabled`, `IsVerticalRailEnabled`, `ZoomMode`, `UseSystemFocusVisuals` | Modes of WinUI's scroll viewer. | Left out. | G-19. |
| SCV-7 | (not in the source) | Touch scrolling is done by the platform. | `ScrollGestureRecognizer` and the snap point bindings on the presenter, as the Fluent theme. | FerroUI's touch scrolling needs them. |
| SCV-8 | `ScrollContentPresenter Margin="{TemplateBinding Padding}"` | The padding of the scroll viewer is a margin around the presenter, outside the scrolled area. | `Padding="{TemplateBinding Padding}"` of the presenter, as the Fluent theme. | FerroUI's presenter takes the padding itself and counts it in the extent, so content laid out to the edge of the padding stays inside the extent (the ControlCatalog's home page test checks it); with a margin the content ends outside the extent. |

## SelectableTextBlock

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| STB-1 | `dxaml/xcp/core/text/common/TextSelectionManager.cpp` (TextBlock with `IsTextSelectionEnabled`) | The selection is drawn in the `SelectionHighlightColor` of the TextBlock, the accent colour of the system by default and the highlight colour of the system in high contrast, with the text in the highlight text colour of the system. | `SelectionBrush` `SystemControlHighlightAccentBrush` (`SystemAccentColor`, `SystemColorHighlightColor` in `HighContrast`), `SelectionForegroundBrush` `SystemColorHighlightTextColor`, the `IBeam` cursor while enabled. | The same colours from the resources of the theme (G-6, G-7). |
| STB-2 | `TextControlCommandBarContextFlyout` (the context flyout of a selectable TextBlock) | Copy and Select All in a command bar flyout. | Pending: a menu flyout of the same commands with batch B3, as for the text box (TXB-9). | RES-6. |

## Separator

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| SEP-1 | CS `MenuFlyout_themeresources.xaml` `{MenuFlyoutSeparator}` | The separator of a menu: a rectangle of `MenuFlyoutSeparatorHeight` in `MenuFlyoutSeparatorBackground` at `MenuFlyoutSeparatorThemePadding` (-4,1,-4,1). | The default theme of FerroUI's Separator, in menus and anywhere else; outside a menu the negative horizontal padding draws the line 4 px past each side of the separator. | FerroUI has one Separator; WinUI separates content outside menus with plain rectangles (and AppBarSeparator in command bars, CommandBarSeparator). |

## TextBlock

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TB-1 | CS `TextBlock_themeresources.xaml`, `TextBlock_themeresources_v2.5.xaml`, GX `HeaderTextBlockStyle`, `SubheaderTextBlockStyle` | Keyed styles of `TextBlock`. | Keyed control themes of `TextBlock` (`Theme=`). | G-16. |
| TB-2 | `LineStackingStrategy="MaxHeight"`, `TextLineBounds="Full"`, `OpticalMarginAlignment="TrimSideBearings"` | Text layout options. | Left out. | G-19. |
| TB-3 | `FontFamily="XamlAutoFontFamily"` | The automatic font. | The family list of G-8. | G-8. |

## TextBox

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TXB-1 | CS `TextBox_themeresources.xaml`, `DefaultTextBoxStyle`: `HeaderContentPresenter`, `DescriptionPresenter` and their rows | Header and description of the control. | Left out. | FerroUI's `TextBox` has no `Header` or `Description`. |
| TXB-2 | `ScrollViewer x:Name="ContentElement"` | The control puts its text view into the scroll viewer. | `PART_ScrollViewer` holding `PART_TextPresenter` (FerroUI's text view), with the text properties bound. | FerroUI's text box requires these parts. |
| TXB-3 | `PlaceholderTextContentPresenter`: foreground bound to `PlaceholderForeground` with the state's theme resource as `TargetNullValue`; visibility set by the control | The placeholder colour follows the state unless the application sets one. | The states set `PlaceholderForeground` of the control (a local value of the application wins); the placeholder is visible while the text and the input method text are empty. | Same behaviour with FerroUI's properties. |
| TXB-4 | `DeleteButton` with the template-local `DeleteButtonStyle`; `ButtonStates` selected by `CTextBox::CanInvokeDeleteButton` (focused, not empty, not read-only, no wrapping, not accepting return, enough width) | A clear button. | `Button` with the keyed theme `TextBoxDeleteButtonStyle`, shown by the selector of the same conditions without the width check, clearing through `Clear` of the control. | FerroUI's text box has no delete button logic; the width check has no selector. |
| TXB-5 | Delete glyph E894 (`TextBoxIconFontSize` 12) | Glyph of the symbol font. | `GlyphGeometryE894`, 10 px. | G-9. |
| TXB-6 | `UseSystemFocusVisuals="{ThemeResource IsApplicationFocusVisualKindReveal}"` | No focus visual (the value is False). | `FocusAdorner="{x:Null}"`. | G-14. |
| TXB-7 | (not in the source) caret colour | The caret follows the text colour. | `CaretBrush="{DynamicResource TextControlForegroundFocused}"`. | FerroUI's caret does not follow the foreground. |
| TXB-8 | `SelectionHighlightColor` | Selection colour. | `SelectionBrush`. | FerroUI's property. |
| TXB-9 | `ContextFlyout="{StaticResource TextControlCommandBarContextFlyout}"`, `SelectionFlyout` | Context and selection command bar flyouts. | Pending: a menu flyout of the same commands with batch B3; no selection flyout. | RES-6. |
| TXB-10 | `Control.IsTemplateFocusTarget`, `IsTabStop="False"` of the scroll viewer, `ZoomMode`, rail and scroll modes | WinUI-specific. | Left out (the scroll viewer is not focusable). | G-19. |
| TXB-11 | (not in the source) | The I-beam cursor is the text view's. | A style of the scroll viewer sets the `IBeam` cursor on its presenter, as the Fluent theme. | FerroUI's text presenter does not set the cursor. |
| TXB-12 | (not in the source) | WinUI 3's text box has no validation. | The template is wrapped in `DataValidationErrors` (the errors under the box, DVE-1); under `:error` the border is `SystemFillColorCriticalBrush`, after the states, so also when focused. | FerroUI's text box reports validation errors through `DataValidationErrors`. |

## TextSelectionHandle

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TSH-1 | `dxaml/xcp/core/text/common/Gripper.cpp`, `TextSelectionSettings.cpp` | The grippers are drawn in code: an outer circle of 4.8 mm, an inner circle of 3.7 mm outlined 0.4 mm, a pole of 0.6 mm, a hit area of 8 mm by 9.3 mm. | A template of the same shapes at 96 dots per inch: circles of 18.14 px and 13.98 px, an outline of 1.51 px, a hit area of 30.24 px by 35.15 px, `PART_Indicator` at the top centre. | FerroUI's TextSelectionHandle is a templated control. |
| TSH-2 | `CTextSelectionGripper::CheckAndRepositionShapesForPoleHeightChange` | The pole runs from the top of the text line down into the circles. | Left out. | FerroUI places the top of the handle at the bottom of the text line and does not give it the height of the line. |
| TSH-3 | `CTextSelectionGripper::CheckAndAdjustStrokeColor`, `CCoreServices::GetTextSelectionGripperBrush` | The outline is the selection colour of the text control made opaque; the fill is white, or `SystemColorWindowBrush` in high contrast. | The outline is `TextControlSelectionHighlightColor` as it is; the fill is `SystemColorWindowColor`. | Markup cannot remove the alpha of a brush; `SystemColorWindowColor` is white in the light and dark dictionaries and the window colour in the high contrast one (G-6). |
| TSH-4 | The selection grippers of WinUI | Start, end and caret grippers are the same circle under the selection point. | The same template for `:caret`, `:start` and `:end`. | Faithful; recorded because FerroUI's other themes draw different start and end handles. |

## ThemeVariantScope

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TVS-1 | (no counterpart) `RequestedTheme` of an element | Needs no style. | Text in the scope takes `TextFillColorPrimaryBrush` of its variant. | Built from WinUI resources. |

## Thumb

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TH-1 | GX `{Thumb}`: zero-length opacity animations of `BackgroundPointerOver`, `BackgroundPressed`, `Background` | Shows one of three borders. | Opacity setters; `:pressed` also hides the pointer-over border, which WinUI's single state does implicitly. | G-1, G-2. |

## TimePicker

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TP-1 | CS `TimePicker_themeresources.xaml`, `{TimePicker}`: `FlyoutButton`, `FlyoutButtonContentGrid`, `FirstPickerHost`, `SecondPickerHost`, `ThirdPickerHost`, `HourTextBlock`, `MinuteTextBlock`, `PeriodTextBlock`, `FirstColumnDivider`, `SecondColumnDivider` | Three columns: hour, minute, period. | `PART_FlyoutButton`, `PART_FlyoutButtonContentGrid`, `PART_FirstPickerHost`, `PART_SecondPickerHost`, `PART_ThirdPickerHost`, `PART_HourTextBlock`, `PART_MinuteTextBlock`, `PART_PeriodTextBlock`, `PART_FirstColumnDivider`, `PART_SecondColumnDivider`, and for the seconds of FerroUI `PART_SecondTextBlock` in the third host, `PART_ThirdColumnDivider` and `PART_FourthPickerHost` (the period). The control sets the columns and the visible hosts. | FerroUI's TimePicker can show seconds (`UseSeconds`). |
| TP-2 | `TimePickerFlyout` (created by `TimePicker_Partial.cpp`) | A flyout the control opens over the button. | `Popup PART_Popup` with `TimePickerPresenter PART_PickerPresenter` in the template. | As DP-2. |
| TP-3 | `TEXT_TIMEPICKER_HOUR_PLACEHOLDER`, `TEXT_TIMEPICKER_MINUTE_PLACEHOLDER`, `TEXT_TIMEPICKER_PERIOD_PLACEHOLDER` | The control shows "hour", "minute" and "AM" without a time. | "hour", "minute" and "second" are the texts of the template; FerroUI's control sets the period to the designator of the current time. | As DP-3; the period is set by FerroUI's control. |
| TP-4 | Template-local `TimePickerFlyoutButtonStyle` | The style of the flyout button. | Keyed theme `TimePickerFlyoutButtonStyle`, as DP-4. | G-18. |
| TP-5 | `IsTabStop="False"`, `UseSystemFocusVisuals`, `FocusVisualMargin="-3"` given to the flyout button; `HeaderContentPresenter`; the first `FocusVisualMargin="0"` setter | As DatePicker. | As DP-5, DP-6. | As DatePicker. |
| TP-6 | `MinWidth="{StaticResource TimePickerThemeMinWidth}"`, `MaxWidth="{StaticResource TimePickerThemeMaxWidth}"` on the flyout button | The sizes of the button, which an application cannot change. | Setters `MinWidth` and `MaxWidth` of the control (the same resources), bound to the button. | FerroUI's TimePicker honours its own `MinWidth` and `MaxWidth`, as the Fluent theme. |
| TP-7 | (not in the source) | The text blocks are top aligned in the button. | The text blocks follow `VerticalContentAlignment` of the control. | As DP-7. |
| TP-8 | `{TimePickerFlyoutPresenter}`: the `LoopingSelector`s that `TimePickerFlyoutPresenter_Partial.cpp` puts into `FirstPickerHost`, `SecondPickerHost`, `ThirdPickerHost` | Looping lists with up and down buttons. | The host panels `PART_HourHost`, `PART_MinuteHost`, `PART_SecondHost`, `PART_PeriodHost` with the template of `DefaultLoopingSelectorStyle` (DTP-5) around the `DateTimePickerPanel`s; `PickerHostGrid` is `PART_PickerContainer`; `PART_ThirdSpacer` divides the seconds. The presenter sets the columns. | FerroUI's presenter requires the hosts and the panels. |
| TP-9 | `HighlightRect` above the items and three `MonochromaticOverlayPresenter`s (margin 3,0, with the background of the highlight) | The highlight covers the selected items, drawn again in `TimePickerFlyoutPresenterHighlightForegroundColor`. | `Border HighlightRect` under the items; the selected items in `TextOnAccentAAFillColorPrimaryBrush` (DTP-8). | As DP-10. |
| TP-10 | `AcceptButton`, `DismissButton` glyphs E8FB, E711 at 16 px | Glyphs of the symbol font. | Paths of `GlyphGeometryE8FB` (14 px) and `GlyphGeometryE711` (12 px). | G-9. |
| TP-11 | The `ThemeShadow` of the flyout, `AutomationProperties.AutomationId`, `TimePickerLightDismissOverlayBackground` | Shadow, automation id, overlay. | Left out. | As DP-12, DP-13. |
| TP-12 | (not in the source) the error state and `DataValidationErrors` of the Fluent theme | WinUI 3 has no input validation. | No error state and no `DataValidationErrors` wrapper. | As CDP-11. |

## ToggleButton

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TGB-1 | CS `ToggleButton_themeresources.xaml`, `DefaultToggleButtonStyle`: `CommonStates` combining pointer and check states (`CheckedPointerOver`, ...) | One group. | Nested styles on combined pseudo-classes in the order of the source. | G-1. |
| TGB-2 | `CheckedDisabled` does not set `BackgroundSizing` (which `Checked` sets to `ToggleButtonCheckedStateBackgroundSizing`) | The base value applies when checked and disabled. | `^:checked:disabled` sets `BackgroundSizing` back to `{TemplateBinding BackgroundSizing}`. | Both `:checked` and `:checked:disabled` match. |
| TGB-3 | `ContentPresenter x:Name="ContentPresenter"`, `BackgroundTransition`, `FocusVisualMargin="-3"` | As Button. | As BTN-2, BTN-3, BTN-5. | As Button. |

## ToolTip

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TT-1 | CS `ToolTip_themeresources.xaml` `DefaultToolTipStyle`: `ContentPresenter x:Name="LayoutRoot"` | The presenter of the template. | `PART_ContentPresenter`. | The part a content control hosts its content in. |
| TT-2 | `OpenStates`: `FadeInThemeAnimation` (Opened), `FadeOutThemeAnimation` (Closed) of `LayoutRoot` | Fades the presenter in and out. | The `Opacity` of the tool tip is 0, and 1 under `:open`, with a linear transition of 250 ms (the fade-in of the Windows animation library); the fade-out is not seen. | G-22; FerroUI closes the popup of a tool tip at once. The opacity is the control's, so that a tool tip shown in a page with `Opacity="1"` is visible. |
| TT-3 | `dxaml/xcp/dxaml/lib/ToolTip_Partial.cpp` (`ApplyElevationEffect` on `LayoutRoot`) | A ThemeShadow under the open tool tip. | No shadow. | The popup of a tool tip is the size of the tool tip and would clip a shadow. |

## TransitioningContentControl

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TCC-1 | GX `{ContentControl}` | WinUI has no transitioning content control; a ContentControl animates a change of content with its `ContentTransitions`. | The template of ContentControl twice in a panel, `PART_ContentPresenter` and `PART_ContentPresenter2`, which FerroUI's control swaps; its `PageTransition` keeps the default of the control (a cross-fade). | FerroUI's control runs its page transition between two presenters. |
| TCC-2 | GX `{ContentControl}`: `HorizontalContentAlignment` Left, `VerticalContentAlignment` Top | A content control does not stretch its content. | Not set: the content stretches (FerroUI's default). | The control hosts pages and views that fill it, as WinUI's Frame does. |
| TCC-3 | GX `{ContentControl}` template | The presenter at the padding; no background or border. | The presenters take the padding, the content alignments, the background and the border. | As LBL-2. |
| TCC-4 | GX `{ContentPresenter}` (`HorizontalAlignment` Left, `VerticalAlignment` Top) | The default alignments of every ContentPresenter. | Waived. | FerroUI's ContentPresenter is not templated; a style of every presenter would be a global style. The templates of the port set the alignments of their presenters. |

## Window

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| WIN-1 | (no counterpart) WinUI 3's `Window` is not a control and has no style | The system draws the window with Mica (or the theme background). | The template FerroUI's window requires (from the Fluent theme) with `SolidBackgroundFillColorBaseBrush` (the Mica fallback), `TextFillColorPrimaryBrush` and the control font. | Built from WinUI resources; G-13. |

## FerroUI controls without a WinUI counterpart

What each FerroUI control the WinUI theme styles, or will style, is built from when WinUI 3 has no
style for it (from `scripts/winui-theme/controls.json`; the deviations of the ported ones are in
their sections above).

<!-- counterparts:begin (generated by scripts/winui-theme/status.py from scripts/winui-theme/controls.json; do not edit) -->

| FerroUI control | Batch | State | Built from |
|---|---|---|---|
| AdornerLayer | F | ported | No WinUI style: the system focus visual of WinUI (FocusVisualPrimaryBrush/Thickness, FocusVisualSecondaryBrush/Thickness; the SystemControlFocusVisual* resources of Common_themeresources.xaml). |
| ButtonSpinner | B2 | pending | No WinUI control: the spin buttons of NumberBox (NumberBoxSpinButtonStyle, NumberBox.xaml) and the TextBox resources. |
| CarouselPage | B5 | pending | No WinUI control: built from FlipView (DefaultFlipViewStyle) and the page background (SolidBackgroundFillColorBaseBrush). |
| ContentPage | B5 | pending | No WinUI style (Page has none): the page background and text of the window (SolidBackgroundFillColorBaseBrush, TextFillColorPrimaryBrush). |
| DataValidationErrors | B1 | ported | No WinUI style (input validation was removed from WinUI 3): the critical fill of the palette (SystemFillColorCriticalBrush) for the message and the border of the field, and the error icon of InfoBar (InfoBar.xaml, InfoBar_themeresources.xaml) for the tool tip variant. |
| DrawerPage | B5 | pending | No WinUI control: the pane of NavigationView (NavigationView_themeresources.xaml) and SplitView (DefaultSplitViewStyle). |
| EmbeddableControlRoot | F | ported | No WinUI style: the background and text of the window (SolidBackgroundFillColorBaseBrush, TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). |
| GridSplitter | B1 | ported | No WinUI control in the merged theme: built from the control fills of the palette (ControlStrongFillColorDefaultBrush, ControlStrongFillColorDisabledBrush) and the accent fill for the preview (AccentFillColorDefaultBrush). |
| GroupBox | B1 | ported | No WinUI control: an expanded Expander without its chevron (DefaultExpanderStyle, Expander.xaml; the header of ExpanderHeaderDownStyle, Expander_themeresources.xaml). |
| HeaderedContentControl | B1 | ported | No WinUI style: the header presenter of the WinUI controls with a Header (TextBox: HeaderContentPresenter, TextBoxTopHeaderMargin, TextControlHeaderForeground). |
| ItemsControl | B1 | ported | No WinUI style in the merged theme (ItemsControl has a built-in template): an ItemsPresenter of the ItemsPanel. |
| Label | B1 | ported | No WinUI control: the implicit style of ContentControl ({ContentControl}, generic.xaml), which Controls/Label.xaml ports. |
| ManagedFileChooser | B6 | pending | No WinUI control: built from the ListViewItem (ListViewItem_themeresources.xaml), TextBox, Button and ContentDialog (DefaultContentDialogStyle) styles. |
| NavigationPage | B5 | pending | No WinUI control: NavigationBackButtonNormalStyle, NavigationBackButtonSmallStyle (NavigationBackButton.xaml) and the page background. |
| OverlayPopupHost | F | ported | No WinUI style: the text of the window (TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). |
| PathIcon | B1 | ported | No WinUI style (IconElement has no template): the Path that WinUI's PathIcon builds in code (CPathIcon::ApplyTemplate, icon.cpp), unscaled and filled with the Foreground. |
| PopupRoot | F | ported | No WinUI style: the text of the window (TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). |
| SelectableTextBlock | B1 | ported | No WinUI control (TextBlock.IsTextSelectionEnabled): the selection colours of the text selection manager of WinUI (SystemControlHighlightAccentBrush, SystemColorHighlightTextColor). |
| TabbedPage | B5 | pending | No WinUI control: TabView (DefaultTabViewStyle) and the page background. |
| TableView | B6 | pending | No control of the merged theme: WinUI's TableView of the tabular library (controls/dev/TableView, TableView_themeresources.xaml, not part of XamlControlsResources) is the closest source; the ListViewItem resources otherwise. |
| TextSelectionHandle | B1 | ported | No WinUI style (the selection grippers are drawn in code, Gripper.cpp): the gripper circles in SystemColorWindowColor outlined in TextControlSelectionHighlightColor. |
| ThemeVariantScope | F | ported | No WinUI style: the text foreground (TextFillColorPrimaryBrush). |
| TransitioningContentControl | B1 | ported | No WinUI style: the template of ContentControl ({ContentControl}, generic.xaml), twice, for the page transition of the control. |
| Window | F | ported | No WinUI style (Window is not a control in WinUI 3): the Mica fallback background and text of a window (SolidBackgroundFillColorBaseBrush, TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). |
| WindowNotificationManager | B6 | pending | No WinUI control: a host of InfoBar cards (InfoBar_themeresources.xaml). |

<!-- counterparts:end -->
