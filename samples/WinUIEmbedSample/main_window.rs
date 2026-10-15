//! `MainWindow` of the reference's sample: a window of WinUI with two columns, controls of WinUI
//! on the left and the swap chain panel of the integration on the right, whose content is the
//! embedded view of the framework.
//!
//! The reference's markup (`MainWindow.xaml`) is compiled by the XAML compiler of the SDK, which
//! a Rust program does not have. The window here loads the same markup at run time
//! (`XamlReader.Load`, [`MARKUP`]) with three changes that loading at run time asks for: the
//! markup is the content of the window (the grid; the title is set in code), the handlers the
//! reference names in the markup (`Click`, `ValueChanged`, `DragStarting`, `SelectionChanged`) are
//! added in code to the elements of their names (a loaded document cannot name the methods of a
//! class), and the panel, which is not a type XAML knows, is added to the second column in code.
//! The texts that name the upstream project name this one. The theme resources the markup asks
//! for (`ApplicationPageBackgroundThemeBrush`, `SystemControlForegroundBaseMediumLowBrush`, ...)
//! are those of `XamlControlsResources`, which the application merges into its resources
//! (`ferroui_winui::XamlApplication`), as the `App.xaml` of the reference does.

use crate::app::App;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::application_lifetimes::ISingleViewApplicationLifetime;
use ferroui_controls::Application;
use ferroui_microcom::{ComPtr, HResult};
use ferroui_win32::win_rt::{HStringInterop, NativeWinRTMethods};
use ferroui_winui::interop::event_handlers::{
    drag_starting_handler, loading_handler, range_base_value_changed_handler, routed_event_handler, selection_changed_handler,
};
use ferroui_winui::interop::{
    DataPackageOperation, IAutomationPeer, IButtonBase, IFrameworkElement, IFrameworkElementAutomationPeerStatics,
    IGridStatics, IInspectable, IRangeBase, ISelector, ITextBlock, ITextBox, IUIElement, IXamlReaderStatics,
};
use ferroui_winui::{FerroSwapChainPanel, XamlWindow};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A handler of two arguments that holds the window weakly and runs the body while the window
/// is alive.
macro_rules! weakly {
    ($weak:expr, |$this:ident| $body:expr) => {{
        let weak = $weak.clone();
        move |_, _| {
            if let Some($this) = weak.upgrade() {
                $body;
            }
        }
    }};
}

/// The content of `MainWindow.xaml` of the reference, loaded at run time (see the module).
const MARKUP: &str = r#"<Grid
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
        <Grid.ColumnDefinitions>
            <ColumnDefinition Width="*" />
            <ColumnDefinition Width="*" />
        </Grid.ColumnDefinitions>

        <!-- Left: native WinUI controls -->
        <Border Grid.Column="0" Background="{ThemeResource ApplicationPageBackgroundThemeBrush}">
            <ScrollViewer Padding="16">
                <StackPanel Spacing="8">
                    <TextBlock Text="WinUI"
                               FontSize="22"
                               FontWeight="SemiBold"
                               Margin="0,0,0,4" />

                    <Button x:Name="WinUiButton"
                            Content="WinUI button"
                            HorizontalAlignment="Stretch" />
                    <TextBlock x:Name="WinUiClickCount" Text="Clicked 0 times" />

                    <Slider x:Name="WinUiSlider"
                            Minimum="0"
                            Maximum="100"
                            Value="25" />
                    <TextBlock x:Name="WinUiSliderValue" Text="Slider: 25" />

                    <TextBox PlaceholderText="Type something…"
                             Text="Hello from WinUI" />

                    <ToggleSwitch Header="ToggleSwitch" IsOn="True" />

                    <TextBlock Text="Drag source (native WinUI baseline)"
                               FontWeight="SemiBold"
                               Margin="0,16,0,0" />
                    <TextBlock Text="Drag the box below — payload: 'Hello from WinUI'. Drop into Notepad, this TextBox, or the FerroUI drop target. Use this to compare against the FerroUI drag source."
                               TextWrapping="Wrap"
                               Opacity="0.7"
                               FontSize="12" />
                    <Border x:Name="WinUiDragSource"
                            CanDrag="True"
                            Height="40"
                            BorderThickness="1"
                            CornerRadius="4"
                            Padding="8"
                            BorderBrush="{ThemeResource SystemControlForegroundBaseMediumLowBrush}"
                            Background="{ThemeResource SystemControlBackgroundAltHighBrush}">
                        <TextBlock Text="Drag me (native WinUI)"
                                   VerticalAlignment="Center" />
                    </Border>

                    <TextBlock Text="Theme" Margin="0,8,0,0" />
                    <ComboBox x:Name="WinUiThemeCombo"
                              SelectedIndex="2"
                              HorizontalAlignment="Stretch">
                        <x:String>Light</x:String>
                        <x:String>Dark</x:String>
                        <x:String>System</x:String>
                    </ComboBox>

                    <TextBlock Text="Automation diagnostics"
                               FontWeight="SemiBold"
                               Margin="0,16,0,0" />
                    <TextBlock Text="Verifies the WinUI automation peer over the FerroUI content. Click Probe peer at different lifecycle points to confirm reachability; the scale readout updates live."
                               TextWrapping="Wrap"
                               Opacity="0.7"
                               FontSize="12" />
                    <Button x:Name="ProbePeerButton"
                            Content="Probe peer + log children"
                            HorizontalAlignment="Stretch" />
                    <Button x:Name="ProbeDeepButton"
                            Content="Probe deep (walk full tree)"
                            HorizontalAlignment="Stretch" />
                    <Button x:Name="ProbeWorkerButton"
                            Content="Probe deep on worker thread"
                            HorizontalAlignment="Stretch" />
                    <TextBlock x:Name="ScaleReadout"
                               Text="Scales: (waiting for Loaded)"
                               FontFamily="Consolas"
                               FontSize="12"
                               TextWrapping="Wrap" />
                    <TextBox x:Name="DiagnosticsLog"
                             IsReadOnly="True"
                             AcceptsReturn="True"
                             TextWrapping="Wrap"
                             FontFamily="Consolas"
                             FontSize="11"
                             Height="160"
                             Text="" />
                </StackPanel>
            </ScrollViewer>
        </Border>

        <!-- Right: FerroUI content embedded via FerroSwapChainPanel (added in code) -->
    </Grid>"#;

/// `ElementTheme` of WinUI.
const ELEMENT_THEME_DEFAULT: i32 = 0;
const ELEMENT_THEME_LIGHT: i32 = 1;
const ELEMENT_THEME_DARK: i32 = 2;

/// A string handle with the text.
fn hstring(text: &str) -> Result<HStringInterop, HResult> {
    HStringInterop::new(Some(text))
}

/// The text of a string handle a method returned (the caller's, deleted here).
fn take_string(handle: isize) -> String {
    HStringInterop::from_handle(handle, true).value().unwrap_or_default()
}

/// `{value:F0}`: the number rounded to an integer, halves away from zero.
fn fixed_0(value: f64) -> String {
    format!("{:.0}", value.round())
}

/// `GetType().Name` of a projected object: the last part of its runtime class name.
fn type_name(object: &IInspectable) -> String {
    let name = object.get_runtime_class_name().map(take_string).unwrap_or_default();
    name.rsplit('.').next().unwrap_or_default().to_string()
}

/// `DateTime.Now.ToString("HH:mm:ss.fff")`.
fn now() -> String {
    // SAFETY: fills a structure of this frame.
    let time = unsafe {
        let mut time = std::mem::zeroed::<windows_sys::Win32::Foundation::SYSTEMTIME>();
        windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut time);
        time
    };
    format!("{:02}:{:02}:{:02}.{:03}", time.wHour, time.wMinute, time.wSecond, time.wMilliseconds)
}

/// An automation peer handed to a thread of its own for the walk of the worker probe. The UI
/// thread keeps a counted reference until the walk has ended ([`MainWindow::probe_worker`]); the
/// worker only calls it, as UI Automation calls the peers from its own threads.
struct PeerOnWorker(usize);

// SAFETY: the pointer is only called on the worker; the reference that keeps the object alive
// stays on the UI thread until the worker has ended with it.
unsafe impl Send for PeerOnWorker {}

pub struct MainWindow {
    window: XamlWindow,
    root: ComPtr<IFrameworkElement>,
    ferro_panel: Rc<FerroSwapChainPanel>,
    clicks: Cell<i32>,
    win_ui_click_count: ComPtr<ITextBlock>,
    win_ui_slider: ComPtr<IRangeBase>,
    win_ui_slider_value: ComPtr<ITextBlock>,
    win_ui_theme_combo: ComPtr<ISelector>,
    diagnostics_log: ComPtr<ITextBox>,
    /// The peers the worker probes walk, until their walks have ended.
    worker_peers: RefCell<Vec<ComPtr<IAutomationPeer>>>,
    /// The lines of the diagnostics are printed too (a smoke run).
    print_diagnostics: Cell<bool>,
}

impl MainWindow {
    pub fn new(print_diagnostics: bool) -> Result<Rc<MainWindow>, HResult> {
        // InitializeComponent()
        let window = XamlWindow::new()?;
        window.set_title("WinUI + FerroUI side by side")?;
        let reader = NativeWinRTMethods::create_activation_factory::<IXamlReaderStatics>("Microsoft.UI.Xaml.Markup.XamlReader")?;
        let root = reader.load(hstring(MARKUP)?.handle())?.ok_or(HResult::POINTER)?.cast::<IFrameworkElement>()?;
        let ferro_panel = FerroSwapChainPanel::new()?;
        let panel_element = ferro_panel.as_ui_element().cast::<IFrameworkElement>()?;
        let grid_statics = NativeWinRTMethods::create_activation_factory::<IGridStatics>("Microsoft.UI.Xaml.Controls.Grid")?;
        grid_statics.set_column(Some(&panel_element), 1)?;
        root.cast::<ferroui_winui::interop::IPanel>()?.children()?.ok_or(HResult::POINTER)?.append(Some(&ferro_panel.as_ui_element()))?;
        window.set_content(&*root.cast::<IUIElement>()?)?;

        let find = |name: &str| -> Result<ComPtr<IInspectable>, HResult> {
            root.find_name(hstring(name)?.handle())?.ok_or(HResult::POINTER)
        };
        let this = Rc::new(MainWindow {
            win_ui_click_count: find("WinUiClickCount")?.cast()?,
            win_ui_slider: find("WinUiSlider")?.cast()?,
            win_ui_slider_value: find("WinUiSliderValue")?.cast()?,
            win_ui_theme_combo: find("WinUiThemeCombo")?.cast()?,
            diagnostics_log: find("DiagnosticsLog")?.cast()?,
            window,
            root: root.clone(),
            ferro_panel,
            clicks: Cell::new(0),
            worker_peers: RefCell::new(Vec::new()),
            print_diagnostics: Cell::new(print_diagnostics),
        });

        // The handlers the markup of the reference names. They hold the window weakly: the
        // elements hold their handlers, and the window holds the elements.
        let weak = Rc::downgrade(&this);
        find("WinUiButton")?.cast::<IButtonBase>()?.add_click(Some(&routed_event_handler(weakly!(weak, |this| this.win_ui_button_click()))))?;
        let on_value_changed = weak.clone();
        this.win_ui_slider.add_value_changed(Some(&range_base_value_changed_handler(move |_, e| {
            if let (Some(this), Some(e)) = (on_value_changed.upgrade(), e) {
                this.win_ui_slider_value_changed(e.new_value().unwrap_or_default());
            }
        })))?;
        find("WinUiDragSource")?.cast::<IUIElement>()?.add_drag_starting(Some(&drag_starting_handler(|_, e| {
            if let Some(e) = e {
                Self::win_ui_drag_source_drag_starting(e);
            }
        })))?;
        this.win_ui_theme_combo
            .add_selection_changed(Some(&selection_changed_handler(weakly!(weak, |this| this.win_ui_theme_combo_selection_changed()))))?;
        find("ProbePeerButton")?.cast::<IButtonBase>()?.add_click(Some(&routed_event_handler(weakly!(weak, |this| this.probe_peer_button_click()))))?;
        find("ProbeDeepButton")?.cast::<IButtonBase>()?.add_click(Some(&routed_event_handler(weakly!(weak, |this| this.probe_deep_button_click()))))?;
        find("ProbeWorkerButton")?.cast::<IButtonBase>()?.add_click(Some(&routed_event_handler(weakly!(weak, |this| this.probe_worker_button_click()))))?;

        // The constructor of the reference.
        let main_view = winui_embed_sample::EmbeddedView::new();
        App::lifetime().set_main_view(Some(main_view.upcast()));
        this.ferro_panel.set_content(App::lifetime().main_view());

        // Probe the automation peer *before* the panel is loaded — answers the open question of
        // whether GetChildren() works before the root is prepared.
        this.append_diagnostic(&format!("ctor: {}", this.describe_peer()));

        panel_element.add_loading(Some(&loading_handler(weakly!(weak, |this| {
            this.append_diagnostic(&format!("Loading: {}", this.describe_peer()))
        }))))?;
        panel_element.add_loaded(Some(&routed_event_handler(weakly!(weak, |this| {
            this.append_diagnostic(&format!("Loaded: {}", this.describe_peer()))
        }))))?;
        Ok(this)
    }

    pub fn activate(&self) -> Result<(), HResult> {
        self.window.activate()
    }

    pub fn window(&self) -> &XamlWindow {
        &self.window
    }

    /// The panel of the second column of the markup of the reference.
    pub fn ferro_panel(&self) -> &Rc<FerroSwapChainPanel> {
        &self.ferro_panel
    }

    pub fn win_ui_slider(&self) -> &ComPtr<IRangeBase> {
        &self.win_ui_slider
    }

    pub fn win_ui_slider_value(&self) -> String {
        self.win_ui_slider_value.text().map(take_string).unwrap_or_default()
    }

    pub fn win_ui_theme_combo(&self) -> &ComPtr<ISelector> {
        &self.win_ui_theme_combo
    }

    /// The requested theme of the content of the window (`ElementTheme`).
    pub fn requested_theme(&self) -> i32 {
        self.root.requested_theme().unwrap_or(-1)
    }

    pub fn diagnostics(&self) -> String {
        self.diagnostics_log.text().map(take_string).unwrap_or_default()
    }

    /// `DescribePeer()`.
    pub fn describe_peer(&self) -> String {
        let described = (|| -> Result<String, HResult> {
            let statics = NativeWinRTMethods::create_activation_factory::<IFrameworkElementAutomationPeerStatics>(
                "Microsoft.UI.Xaml.Automation.Peers.FrameworkElementAutomationPeer",
            )?;
            let Some(peer) = statics.create_peer_for_element(Some(&self.ferro_panel.as_ui_element()))? else {
                return Ok("peer=null".to_string());
            };
            let children = peer.get_children()?;
            let count = match &children {
                Some(children) => children.size()?,
                None => 0,
            };
            let mut sb = format!(
                "peer={} class={} childCount={count}",
                type_name(&*peer.cast::<IInspectable>()?),
                take_string(peer.get_class_name()?)
            );
            if let (Some(children), true) = (&children, count > 0) {
                let first = children.get_at(0)?.ok_or(HResult::POINTER)?;
                let first_children = match first.get_children()? {
                    Some(first_children) => first_children.size()?,
                    None => 0,
                };
                sb.push_str(&format!(
                    " first=[{}/{:?}] firstChildren={first_children}",
                    take_string(first.get_class_name()?),
                    first.get_automation_control_type()?
                ));
            }
            Ok(sb)
        })();
        described.unwrap_or_else(|error| format!("threw: {error}"))
    }

    fn probe_peer_button_click(&self) {
        self.append_diagnostic(&format!("Probe: {}", self.describe_peer()));
    }

    /// `async void`: the walk on a thread of its own, the line on the UI thread when it has
    /// ended.
    fn probe_worker_button_click(&self) {
        // Cache the peer on the UI thread (CreatePeerForElement is UI-thread bound).
        let peer = NativeWinRTMethods::create_activation_factory::<IFrameworkElementAutomationPeerStatics>(
            "Microsoft.UI.Xaml.Automation.Peers.FrameworkElementAutomationPeer",
        )
        .and_then(|statics| statics.create_peer_for_element(Some(&self.ferro_panel.as_ui_element())));
        let peer = match peer {
            Ok(Some(peer)) => peer,
            Ok(None) => {
                self.append_diagnostic("Worker-thread walk: peer=null");
                return;
            }
            Err(error) => {
                self.append_diagnostic(&format!("Worker-thread walk: threw: {error}"));
                return;
            }
        };
        let on_worker = PeerOnWorker(peer.as_ptr() as usize);
        self.worker_peers.borrow_mut().push(peer);

        // Walk on a worker thread — mimics how UIA queries (it calls from a separate thread).
        let spawned = std::thread::Builder::new().name("worker-thread walk".to_owned()).spawn(move || {
            let on_worker = on_worker;
            // SAFETY: the thread id of this thread.
            let thread_id = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
            let mut sb = format!("Worker-thread walk (TID={thread_id}):\n");
            // SAFETY: the peer the UI thread keeps until this walk has ended (above).
            let peer = unsafe { &*(on_worker.0 as *const IAutomationPeer) };
            if let Err(error) = walk_peer(peer, 0, &mut sb, 20) {
                sb.push_str(&format!("THREW: {error}"));
            }
            let address = on_worker.0;
            Dispatcher::ui_thread().post(
                move || {
                    App::with_main_window(|window| {
                        window.worker_peers.borrow_mut().retain(|peer| peer.as_ptr() as usize != address);
                        window.append_diagnostic(&sb);
                    });
                },
                DispatcherPriority::NORMAL,
            );
        });
        if let Err(error) = spawned {
            self.append_diagnostic(&format!("Worker-thread walk: threw: {error}"));
        }
    }

    fn probe_deep_button_click(&self) {
        let walked = (|| -> Result<String, HResult> {
            let statics = NativeWinRTMethods::create_activation_factory::<IFrameworkElementAutomationPeerStatics>(
                "Microsoft.UI.Xaml.Automation.Peers.FrameworkElementAutomationPeer",
            )?;
            let peer = statics.create_peer_for_element(Some(&self.ferro_panel.as_ui_element()))?;
            let mut sb = String::from("Deep walk:\n");
            if let Some(peer) = &peer {
                walk_peer(peer, 0, &mut sb, 20)?;
            }
            Ok(sb)
        })();
        match walked {
            Ok(sb) => self.append_diagnostic(&sb),
            Err(error) => self.append_diagnostic(&format!("Deep probe threw: {error}")),
        }
    }

    /// What the button "Probe deep on worker thread" does.
    pub fn probe_worker(&self) {
        self.probe_worker_button_click();
    }

    /// The deep walk of the probe on the UI thread.
    pub fn deep_walk(&self) -> String {
        self.probe_deep_button_click();
        self.diagnostics()
    }

    /// `AppendDiagnostic(line)`.
    pub fn append_diagnostic(&self, line: &str) {
        let text = self.diagnostics_log.text().map(take_string).unwrap_or_default();
        let text = format!("{text}[{}] {line}\n", now());
        if let Ok(text) = hstring(&text) {
            let _ = self.diagnostics_log.set_text(text.handle());
        }
        if self.print_diagnostics.get() {
            println!("[diag] {line}");
        }
    }

    fn win_ui_button_click(&self) {
        self.clicks.set(self.clicks.get() + 1);
        if let Ok(text) = hstring(&format!("Clicked {} times", self.clicks.get())) {
            let _ = self.win_ui_click_count.set_text(text.handle());
        }
    }

    fn win_ui_drag_source_drag_starting(e: &ferroui_winui::interop::IDragStartingEventArgs) {
        if let (Ok(Some(data)), Ok(text)) = (e.data(), hstring("Hello from WinUI")) {
            let _ = data.set_text(text.handle());
        }
        let _ = e.set_allowed_operations(DataPackageOperation(DataPackageOperation::Copy.0 | DataPackageOperation::Move.0));
    }

    fn win_ui_slider_value_changed(&self, new_value: f64) {
        if let Ok(text) = hstring(&format!("Slider: {}", fixed_0(new_value))) {
            let _ = self.win_ui_slider_value.set_text(text.handle());
        }
    }

    fn win_ui_theme_combo_selection_changed(&self) {
        let selected = self.win_ui_theme_combo.selected_index().unwrap_or(-1);

        if let Ok(Some(content)) = self.window.window().content() {
            if let Ok(root) = content.cast::<IFrameworkElement>() {
                let _ = root.set_requested_theme(match selected {
                    0 => ELEMENT_THEME_LIGHT,
                    1 => ELEMENT_THEME_DARK,
                    _ => ELEMENT_THEME_DEFAULT,
                });
            }
        }

        if let Some(application) = Application::current() {
            application.set_requested_theme_variant(Some(match selected {
                0 => ThemeVariant::light(),
                1 => ThemeVariant::dark(),
                _ => ThemeVariant::default(),
            }));
        }
    }
}

/// `WalkPeer(peer, depth, sb, maxDepth)`.
fn walk_peer(peer: &IAutomationPeer, depth: usize, sb: &mut String, max_depth: usize) -> Result<(), HResult> {
    if depth > max_depth {
        return Ok(());
    }

    for _ in 0..depth {
        sb.push_str("  ");
    }
    let class_name = take_string(peer.get_class_name()?);
    let name = take_string(peer.get_name()?);
    let control_type = peer.get_automation_control_type()?;
    let children = peer.get_children()?;
    let count = match &children {
        Some(children) => children.size()?,
        None => 0,
    };
    sb.push_str(&format!("[{class_name}] type={control_type:?} name=\"{name}\" children={count}\n"));

    let Some(children) = children else {
        return Ok(());
    };
    for index in 0..count {
        if let Some(child) = children.get_at(index)? {
            walk_peer(&child, depth + 1, sb, max_depth)?;
        }
    }
    Ok(())
}

