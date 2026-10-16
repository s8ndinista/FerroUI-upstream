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

## CheckBox

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| CHK-1 | CS `CheckBox_themeresources.xaml`, `DefaultCheckBoxStyle`: `Grid x:Name="RootGrid"` with background, border and corner radius | The root panel draws the background and border the states change. | `Border x:Name="RootGrid"` around a `Grid`. | G-17. |
| CHK-2 | `controls:AnimatedIcon x:Name="CheckGlyph"` with `AnimatedAcceptVisualSource` and the fallback `FontIconSource` (`CheckBoxCheckedGlyph` E73E, `CheckBoxIndeterminateGlyph` E9AE, `CheckBoxGlyphSize` 12) | An animated check mark; the fallback glyphs of the symbol font. | A `Path` with `GlyphGeometryE73E` (10 px wide) when checked and `GlyphGeometryE9AE` (8 px wide, margin 0) when indeterminate; hidden (opacity 0) when unchecked. | G-9, G-10. The widths are the estimated ink of the glyphs at 12 px. |
| CHK-3 | The `AnimatedIcon.State` setters of `CombinedStates` (`NormalOff`, `PointerOverOn`, ...) | Select the frame of the animation. | The opacity and data of the path per check state. | G-10. |
| CHK-4 | `ContentPresenter x:Name="ContentPresenter"` | The content presenter. | `PART_ContentPresenter`. | Template part (BTN-2). |
| CHK-5 | `FocusVisualMargin="{StaticResource CheckBoxFocusVisualMargin}"` (-7,-3,-7,-3) | Focus visual around box and label. | `FocusAdorner` with that margin. | G-14. |

## EmbeddableControlRoot

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| ECR-1 | (no counterpart) | WinUI 3 has no root of content hosted in another UI with a style. | The template FerroUI's control requires (from the Fluent theme), with `SolidBackgroundFillColorBaseBrush`, `TextFillColorPrimaryBrush`, `ContentControlThemeFontFamily`, `ControlContentThemeFontSize`. | Built from the resources of a WinUI window. |

## HyperlinkButton

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| HLB-1 | `dxaml/xcp/dxaml/lib/HyperlinkButton_Partial.cpp` (`SetCursor(MouseCursorHand)`) | The control sets the hand cursor in code. | `Cursor="Hand"` setter in `DefaultHyperlinkButtonStyle`. | FerroUI's hyperlink button does not set a cursor itself. |
| HLB-2 | CS `HyperlinkButton_themeresources.xaml` (no visited state) | A visited hyperlink button looks the same. | No style for FerroUI's `:visited`. | Faithful; noted because the Fluent theme styles `:visited`. |
| HLB-3 | `ContentPresenter x:Name="ContentPresenter"`, `BackgroundTransition`, `FocusVisualMargin="-3"` | As Button. | As BTN-2, BTN-3, BTN-5. | As Button. |

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

## ThemeVariantScope

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TVS-1 | (no counterpart) `RequestedTheme` of an element | Needs no style. | Text in the scope takes `TextFillColorPrimaryBrush` of its variant. | Built from WinUI resources. |

## Thumb

| ID | WinUI source | What WinUI does | What the port does | Why |
|---|---|---|---|---|
| TH-1 | GX `{Thumb}`: zero-length opacity animations of `BackgroundPointerOver`, `BackgroundPressed`, `Background` | Shows one of three borders. | Opacity setters; `:pressed` also hides the pointer-over border, which WinUI's single state does implicitly. | G-1, G-2. |

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
| DataValidationErrors | B1 | pending | No WinUI style (input validation was removed from WinUI 3): the error brushes (SystemControlErrorTextForegroundBrush, SystemFillColorCriticalBrush) and the info glyph of InfoBar. |
| DrawerPage | B5 | pending | No WinUI control: the pane of NavigationView (NavigationView_themeresources.xaml) and SplitView (DefaultSplitViewStyle). |
| EmbeddableControlRoot | F | ported | No WinUI style: the background and text of the window (SolidBackgroundFillColorBaseBrush, TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). |
| GridSplitter | B1 | pending | No WinUI control in the merged theme: built from the control fill and stroke resources (ControlStrongFillColorDefaultBrush, ControlFillColorDefaultBrush). |
| GroupBox | B1 | pending | No WinUI control: the card resources (CardBackgroundFillColorDefaultBrush, CardStrokeColorDefaultBrush, OverlayCornerRadius) and the header of Expander. |
| HeaderedContentControl | B1 | pending | No WinUI style: the header presenter of the WinUI controls with a Header (TextBox: HeaderContentPresenter, TextBoxTopHeaderMargin, TextControlHeaderForeground). |
| ItemsControl | B1 | pending | No WinUI style in the merged theme (ItemsControl has a built-in template): an ItemsPresenter. |
| Label | B1 | pending | No WinUI control: the content presenter of a ContentControl ({ContentControl}, generic.xaml) with the body text resources. |
| ManagedFileChooser | B6 | pending | No WinUI control: built from the ListViewItem (ListViewItem_themeresources.xaml), TextBox, Button and ContentDialog (DefaultContentDialogStyle) styles. |
| NavigationPage | B5 | pending | No WinUI control: NavigationBackButtonNormalStyle, NavigationBackButtonSmallStyle (NavigationBackButton.xaml) and the page background. |
| OverlayPopupHost | F | ported | No WinUI style: the text of the window (TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). |
| PathIcon | B1 | ported | No WinUI style (IconElement has no template): the Path that WinUI's PathIcon builds in code (CPathIcon::ApplyTemplate, icon.cpp), unscaled and filled with the Foreground. |
| PopupRoot | F | ported | No WinUI style: the text of the window (TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). |
| SelectableTextBlock | B1 | pending | No WinUI control (TextBlock.IsTextSelectionEnabled): the selection highlight of the text controls (TextControlSelectionHighlightColor). |
| TabbedPage | B5 | pending | No WinUI control: TabView (DefaultTabViewStyle) and the page background. |
| TableView | B6 | pending | No control of the merged theme: WinUI's TableView of the tabular library (controls/dev/TableView, TableView_themeresources.xaml, not part of XamlControlsResources) is the closest source; the ListViewItem resources otherwise. |
| TextSelectionHandle | B1 | pending | No WinUI style (the selection grippers are drawn by the text control): built from the accent fill (AccentFillColorDefaultBrush). |
| ThemeVariantScope | F | ported | No WinUI style: the text foreground (TextFillColorPrimaryBrush). |
| TransitioningContentControl | B1 | pending | No WinUI style: ContentPresenter with content transitions ({ContentPresenter}, generic.xaml). |
| Window | F | ported | No WinUI style (Window is not a control in WinUI 3): the Mica fallback background and text of a window (SolidBackgroundFillColorBaseBrush, TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). |
| WindowNotificationManager | B6 | pending | No WinUI control: a host of InfoBar cards (InfoBar_themeresources.xaml). |

<!-- counterparts:end -->
