# Declarative user interfaces in Rust (`ferroui-view`)

FerroUI user interfaces can be written in Rust without XAML: elements built with chained setters or with the `rsx!` macro, state held in signals that the properties of controls follow, children that follow state, and components that run again whose output a reconciler patches into the controls. The controls, themes, layout, styles and bindings are the ones XAML and imperative Rust use; this is another way to create and connect them.

This document is the design of the crates and the plan of the work (issue #237). Two phases are built:

- **Phase 0**: the element tree, the builders, the macros, fine-grained reactivity, and the 7GUIs samples.
- **Phase 1**: components that run again, their hooks, and the reconciler (sections 7 and 8).

Section 10 plans the phases that remain. [declarative-ui-react.md](declarative-ui-react.md) compares the API with React.

## 1. A first look

The counter of 7GUIs with the builders:

```rust
use ferroui_view::prelude::*;

pub fn counter() -> Element {
    let count = signal(0);
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .child(text_box().text(count).is_read_only(true).width(80.0))
        .child(button().content("Count").on_click(move |_| count.update(|c| *c += 1)))
        .into()
}
```

The same with the macro:

```rust
#[component]
pub fn Counter() -> Element {
    let count = signal(0);
    rsx! {
        StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
            TextBox { text: count, is_read_only: true, width: 80.0 }
            Button { on_click: move |_| count.update(|c| *c += 1), "Count" }
        }
    }
}
```

The same as a component that runs again (phase 1): the function runs again when the count changes, and the reconciler patches the text of the field:

```rust
#[component(rerender)]
pub fn Counter() -> Element {
    let count = use_signal(|| 0);
    rsx! {
        StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
            TextBox { text: count.get().to_string(), is_read_only: true, width: 80.0 }
            Button { on_click: move |_| count.update(|c| *c += 1), "Count" }
        }
    }
}
```

An application whose main window is an element (`ferroui-view` with its feature `desktop`):

```rust
fn main() {
    ferroui_view::launch(|| rsx! { Window { title: "Counter", width: 320.0, height: 120.0, Counter {} } });
}
```

## 2. The crates

| Crate | What it holds |
|---|---|
| `ferroui-view` (`src/FerroUI.View`) | The element tree (`element.rs`), mounting and the reconciler (`mount.rs`), the reactive runtime with contexts (`reactive.rs`), the hooks (`hooks.rs`), the builders (generated), the conversions of values, children that follow state, and with the feature `desktop` the launch of an application |
| `ferroui-view-macros` (`src/FerroUI.View.Macros`) | `rsx!` and `#[component]` (with `rerender` and `memo`) |

`ferroui-view` depends on `ferroui-base` and `ferroui-controls`; it adds nothing to them. The builders are generated from their sources by `scripts/generate_view_builders.py`, so a control or a property ported later gets its builder by running the script again (`--check` fails when the generated file is stale; `--report` lists what was left out and why).

## 3. Elements

An `Element` describes user interface: a control with its property values, handlers and children, text, several elements, children that follow state, a component that runs again, or a control that exists already (a XAML user control, a control the application keeps).

The element of a control is data:

- the class (how to create the control, compared by type);
- the key among its siblings;
- the `Name` and the style classes;
- the property values: a list of the property (`&'static FerroProperty`) and its value, where a plain value is a `BoxedValue` that compares by value;
- the handlers;
- the children.

`Element::mount` creates the controls: each control is created, told that it is being initialised (`begin_init`, as the XAML loader does, so that an items control applies a selected index once its items are there), given its values, handlers and children, and initialised (`end_init`). Children go into the content of the control: `Children` of a panel, `Items` of an items control, `Content` of a content control, `Child` of a decorator, the text of a `TextBlock`. Text is text where the content takes text (a content, an item) and a `TextBlock` elsewhere.

Because the element of a control is data compared by value, the reconciler compares two trees of elements and patches the controls of the first into the second (section 8).

## 4. Builders

For every class of `FerroUI.Base` and `FerroUI.Controls` the generator writes:

- a function that starts an element of a class that can be created (`button()`, `stack_panel()`, 134 of them);
- a trait of the class (`ButtonProps`) with a setter per registered property (`click_mode(..)`) and a handler per routed event (`on_click(..)`), implemented for the element of the class and of every class derived from it (193 traits);
- one trait with a setter per attached property, named after its owner (`grid_row(..)`, `dock_panel_dock(..)`), and the setter on the owner class itself (`scroll_viewer().horizontal_scroll_bar_visibility(..)`).

The element also has its own methods:

| Method | What it does |
|---|---|
| `key(..)`, `name(..)`, `class("a b")` | The key, the `Name`, style classes |
| `child(..)`, `children((a, b, c))` | Children: elements, builders, text, vectors, arrays, tuples |
| `set(property, value)` | Any property, also one the generator left out or one of another crate |
| `bind(property, binding)` | A binding of the property system (`Binding`, a path on the data context) |
| `on(event, handler)` | Any routed event, with the control that handles it |
| `on_changed(property, handler)` | The new value of a property whenever it changes |
| `with(|control| ..)` | The control once it was made (a call the class has no property for) |
| `node_ref(r)` | Keeps the control in a `NodeRef` |
| `row_definitions("Auto,*")`, `column_definitions(..)` | The rows and columns of a grid, as XAML writes them |

### 4.1 What a setter takes

A setter of a property whose value is `T` takes what converts to it (`IntoProp<T, _>`, the second parameter inferred):

| Given | The property |
|---|---|
| a `T`, or a value that converts to `T` (below) | is set once |
| a `Signal<V>` or a `Memo<V>` | follows it |
| a closure `move \|\| ..` | follows what the closure reads |
| `signal.two_way()` | follows the signal, and a change of the property (the user typed, a slider moved) is written into the signal |
| `Bind(binding)` | is bound by the property system |
| an element, for a property that holds an object (`content`, `header`, `tag`) | holds its control |

The conversions of values: `T` to `Option<T>`; text to `String` and `Option<String>`; numbers to `f64` and to text; `f64` and tuples to `Thickness`; `f64` to `CornerRadius`; a `Color`, a color name or `#rgb` text to a brush.

### 4.2 The forms of upstream's code-first API

Upstream writes properties and bindings in code with indexers and operators on properties. Rust has neither an assignable indexer nor a second prefix operator (the port already maps `!Property` to `Property.bind()`, see the waivers of `AvaloniaProperty`), so the element has methods for them:

| Upstream C# | Here |
|---|---|
| `control[Grid.RowProperty] = 1` | `.set(Grid::row_property(), 1)` or `.grid_row(1)`; in `rsx!`, `Grid.row: 1` |
| `control[!TextBlock.TextProperty] = binding` | `.bind(TextBlock::text_property(), binding)` |
| `control.Bind(Property, observable)` | `.text(signal)`, `.text(move \|\| ..)` |
| `!!` (two-way) | `.text(signal.two_way())` |
| `control.Classes.Add("accent")` | `.class("accent")` |
| `new Style(x => x.OfType<Button>())` | the styles API of the port (`Selectors`, `Style`, `Setter`), unchanged |

## 5. The macros

### 5.1 `rsx!`

```rust
rsx! {
    Grid { row_definitions: "Auto,*", column_definitions: "*,Auto",
        TextBox { Grid.row: 0, text: draft.two_way() }
        Button { Grid.column: 1, class: "accent", on_click: add, "Add" }
        ListBox { Grid.row: 1, { for_each(move || todos.get(), |t| t.id, |t| rsx! { ListBoxItem { content: t.title } }) } }
    }
}
```

| Form | Meaning |
|---|---|
| `Name { .. }` | An element of the class or component `Name` in scope: `<Name as Builder>::builder()`. The prelude names every control class; a component declares its own type |
| `name: value` | The setter `.name(value)`: a property, a handler, `key`, `name`, `class` |
| `name` | The setter `.name(name)`, as a field of a struct is given |
| `Owner.name: value` | An attached property: `.owner_name(value)` (`Grid.row`, `DockPanel.dock`, `Canvas.left`) |
| `name(a, b)` | A method with several arguments: `on_changed(property, handler)`, `set(property, value)`, `bind(..)`, `with(..)` |
| `"text"`, `{expr}`, a nested element | Children |
| `if cond { .. } else { .. }`, `for x in items { .. }` | Children chosen or repeated when the tree is built |

An attribute a class has no setter for is a missing method of its builder: the error points at the attribute.

### 5.2 `#[component]`

```rust
#[component]
fn Labelled(label: String, #[prop(default = 2)] digits: usize, value: f64) -> Element { .. }

rsx! { Labelled { label: "Sum", value: total } }     // or labelled().label("Sum").value(total)
```

The function stays as it is. Beside it the macro declares a type of the name (what `rsx!` names), a builder function in snake case (`labelled()`) with a setter per parameter taking what converts into its type, and the key. A parameter is required unless it has `#[prop(default)]`, `#[prop(default = expr)]` or an `Option` type; a missing one panics with its name when the element is built. A parameter `children: Element` takes the children of the element.

By default a component runs once, when its element is built: its function is called by the builder and returns the element in its place. Two options make it a component that runs again (section 7):

| Attribute | The component |
|---|---|
| `#[component]` | runs once; what changes afterwards changes through signals |
| `#[component(rerender)]` | runs when it is mounted and again whenever a signal it read changes or its parent runs again; its parameters are cloned for each run, so they are `Clone` |
| `#[component(rerender, memo)]` | as `rerender`, and a parent that runs again with parameters equal to the last ones (`PartialEq`) does not run it again |

## 6. State

`signal(value)`, `memo(|previous| ..)` and `effect(|| ..)` are fine-grained reactivity: an effect records the signals and memos it reads and runs again when one of them changes; a memo is a value computed the same way and read like a signal. Handles are `Copy`, so a `move` closure takes them without a clone.

Every signal, memo and effect belongs to the `Owner` that was current when it was made, and is disposed with it: a mounted tree has an owner (`Element::mount_root`, a window mounted by `launch` or `open_window` ends it when the window closes), each item of a dynamic list has its own, and an effect owns what it makes while it runs. `on_cleanup` runs code when the owner ends (a timer stopped, a subscription disposed). Using a handle after its owner ended panics. `batch` runs the effects once after several writes; `untrack` reads without following.

### 6.1 Children that follow state

| Function | Children |
|---|---|
| `for_each(each, key, view)` | One per item, kept by key: an item whose key and value stay keeps its control; a new or changed item gets a new one; the controls of items that left are taken out |
| `show(when, then)`, `show_else(when, then, otherwise)` | The children of the branch, made again only when the condition changes |
| `dynamic(render)` | Whatever the closure gives, made again when what it read changes; an element with a key keeps its control |

They are elements a parent mounts in place, among its other children, in panels, items controls and contents.

## 7. Components that run again (phase 1)

A component that runs again is the model of React: its function returns the elements of its state, it runs again when the state changes, and the reconciler patches the controls it made into what it returns now (section 8). It is made with `rerender(|| ..)` with the builders, or with `#[component(rerender)]`:

```rust
fn counter() -> Element {
    rerender(|| {
        let count = use_signal(|| 0);
        button()
            .content(format!("Clicked {} times", count.get()))
            .on_click(move |_| count.update(|c| *c += 1))
            .into()
    })
}
```

**When it runs again.** The function runs inside an effect of the runtime of section 6, so it runs again when a signal or memo it read changes. There is no `set_state` that schedules a run: a write to a signal the function read is that. It also runs again when its parent runs again and gives it its parameters anew (unless it is memoized and they are equal). Two writes in `batch(|| ..)` run it once; without `batch`, each write runs it at once, as the effects of phase 0 do. A handler is not batched on its own: code of phase 0 counts on effects that run in the middle of a handler (the temperature converter, the CRUD selection).

**What runs where.** Reading a signal in the function makes the component run again when it changes. Reading it in a property instead (`text: count`, `text: move || ..`) changes only that property, as in phase 0, and the component does not run again. The two mix freely: the CRUD of the hooks samples keeps the selection two-way (`selected_index: selected.two_way()`) and computes the filtered list in the function.

### 7.1 Hooks

What outlives one run is kept in hooks (`hooks.rs`). Each `use_*` call has a slot of the component, found by the order of the calls. So a hook is called on every run, in the same order; a run that calls another hook than the first run did at the same place panics and names the rule.

| Hook | What it gives |
|---|---|
| `use_hook(|| value)` | A value made on the first run and handed back (cloned) on the next ones: the slot every other hook is built on (a `NodeRef`, an `Rc` of a cache) |
| `use_signal(|| value)` | A signal kept across runs: the state of the component |
| `use_memo(|| ..)` | A memo kept across runs; it follows the signals it reads |
| `use_effect(|| ..)` | An effect started once the controls of the first run are in place; it runs again when the signals it read change |
| `use_effect_with(deps, |deps| ..)` | A closure run once the controls of a run are in place, on the first run and whenever `deps` differ from those of the call before; what it registers with `on_cleanup` runs before the next call and when the component is taken out |
| `provide_context(value)`, `use_context::<T>()` | A value provided to the components below, found by type (also outside components, on the current owner) |

What a hook makes belongs to the component: it is disposed when the component is taken out, and the cleanups run. A signal made with `signal(..)` in the function itself belongs to the run and is made again on the next one; in a component that runs again, state is a hook. For the same reason a `NodeRef` is kept with `use_hook(NodeRef::new)`, and the code of an element's `with(..)` runs when its control is made, not on later runs.

### 7.2 The two models together

| | Runs once (`#[component]`) | Runs again (`#[component(rerender)]`) |
|---|---|---|
| State | `signal(..)` in the function | `use_signal(..)` |
| What follows a change | the properties and children bound to the signal | the function runs again; the reconciler patches the difference |
| Lists and conditions | `for_each`, `show`, `dynamic` | `for` and `if` of `rsx!`, `iter().map(..)`, with `key` on the items of a list |
| Cost of a change | the bound properties only | the function and a comparison of its elements |
| Effects | `effect(..)`, `on_cleanup(..)` | `use_effect`, `use_effect_with` |

Both produce the same elements, so a component of either model is a child of the other. `for_each`, `show` and `dynamic` work in a component that runs again too: when the component runs again, the list takes the new closure, runs again, and patches the items it keeps (section 8.4).

## 8. The reconciler

`mount.rs` holds the mounting of elements and the reconciler that patches mounted controls into a new element tree. This section is its design.

### 8.1 Instances and slots

A mounted element is an instance: the control it made (or the text, the list, the component) with what it was made from, so that the next tree can be compared with it.

| Instance | Holds |
|---|---|
| Control | the class, the key, the control, an owner, and the mounted state: the name, the style classes, the property values with what each started, the owner of the handlers, and the mounted children |
| Text | the text, and the text value or the `TextBlock` it placed |
| Existing | the control it was given |
| Dynamic | the closure of the list, its items by key (each with an owner and the instances it rendered) |
| Component | the type, the key, the closure that runs it, its parameters when memoized, its hook slots, an owner, and the instances it rendered |

Each instance places its controls into the content of the parent control through a slot. A slot is either a list of placed children (a control, or a value such as the text of a content or an item) or a group of slots: the children of a control, the items of a dynamic list, the output of a component. The top slot of a control's content belongs to the host, which is `Children` of a panel, `Items` of an items control, `Content` of a content control or of a property that holds an element, `Child` of a decorator, or the text of a `TextBlock`.

A slot that changes on its own (a dynamic list whose signal changed, a component that ran again) computes where it begins among the host's children by walking up the groups. It replaces its part of the host's children, and only the part that differs: the common prefix and suffix of the old and new lists are left in place, and the middle is removed and inserted. Until the parent placed its children for the first time, a change places nothing and the parent places every child at once.

The root of a mounted tree has no host: its control is handed to the caller (`Element::mount`, a window). A component at the root may patch its root control, but not replace it; a root whose class changes panics with a message that says so.

### 8.2 Matching

`reconcile_list` patches the instances of a slot into a new list of elements:

1. Fragments are flattened and empty elements left out.
2. Each new element with a key takes the old instance with that key. Each new element without a key takes the next old instance without a key, in order: siblings without a key are matched by their place among the siblings without a key.
3. A match of the same kind is patched and keeps its control: the same control class, text and text, the same existing control, two dynamic lists, the same component type (the type of the closure of `rerender`, the type `#[component]` declares). Anything else is mounted anew.
4. The slot takes the new list of instances; the host gets the difference (section 8.1).
5. The instances that were not taken are disposed after they left the host: their owners end, which stops their effects and detaches their handlers.

These are the rules of React's reconciliation (different type: replace; same type: update; keys identify siblings), with one difference: an element without a key is matched by its place among the elements without a key, not by its place among all siblings, so adding a keyed element does not shift the unkeyed ones.

### 8.3 Patching a control

| Part | Rule |
|---|---|
| Name, style classes | set when they differ; classes that are gone are removed and new ones added |
| A property with an equal plain value | left alone: values compare by value (`BoxedValue` equality) |
| A property with another value | what the old value started is stopped (the effect of a reactive value, a two-way subscription, a binding expression), then the new one is applied |
| A reactive value, a two-way value, a binding | never equal (closures are not comparable), so applied again on each run; applying sets the same value, which the property system ignores |
| A property that holds an element (`content`, `header`) | the element is reconciled with the one mounted there, so a content keeps its control |
| A property that is gone | cleared (`clear_value_untyped`), back to its default or style value |
| Handlers | detached and attached again: closures are never equal; a routed handler is removed by its token when its owner ends |
| Children | reconciled by the rules of section 8.2 |
| `with(..)`, `node_ref(..)` | not run again: they run when the control is made |

The patch does not call `begin_init` and `end_init`: they are for a control that is being made.

### 8.4 Dynamic lists and components

**A dynamic list** (`for_each`, `show`, `dynamic`) keeps its own effect and items. When its own signals change, it does what it did in phase 0: items with an equal key and value stay as they are, new or changed items are made, and the others are taken out. When the component that holds it runs again, the list takes the new closure and runs again in refresh mode: the items it keeps render again and are patched with the reconciler, so a closure that captured a new value of the parent shows it. What an item's elements made while rendering belongs to a scope of the item that is replaced on each render.

**A component** has an owner for its hooks and a second owner for the instances it rendered. Its function runs in an effect owned by the component; the elements it returns are reconciled with the instances of the run before, outside the effect (`untrack`), under the owner of the instances. Then the closures that hooks queued run (`use_effect`, `use_effect_with`).

When its parent runs again, the component takes the new closure and its parameters, and a version signal it reads changes. So the component runs once more, after its parent's run, in the same propagation of changes. A memoized component whose parameters are equal skips this.

### 8.5 Lifetimes

Every instance has an owner of the runtime of section 6, made under the owner of its parent. Disposing an owner disposes what it owns, so taking an instance out stops everything below it: effects, bindings, handlers, hooks, nested components. An instance is kept alive by its owner (a cleanup holds it) as well as by its parent, so a slot whose parent was dropped still places its changes. The cleanup releases it when the owner ends, and no reference cycle outlives the owner.

### 8.6 Costs and limits

- **The cost of a run** is the function and a walk of the elements it returned: equal plain values are compared and skipped. Reactive values, bindings and handlers are applied again on each run, so a component that runs often (the timer of the hooks samples runs on every tick) is better off with fine-grained values for what changes most.
- **Moves.** A keyed list whose order changes is patched by the prefix and suffix rule, which removes and inserts the controls in between. That is correct and keeps the controls, but it is not the minimal number of moves (section 10, phase 5).
- **Synchronous.** A run happens when the write that caused it ends (or the batch); there is no scheduler that splits or defers work, and no concurrent rendering.
- **Items controls.** The children of an items control are its items; an items control with virtualization and an item template is not driven by the reconciler (section 10, phase 2).

## 9. Samples: 7GUIs

The seven tasks of 7GUIs (<https://eugenkiss.github.io/7guis/tasks>) are written three times, each task a crate of its own so that each stays small:

- with the builders;
- with `rsx!`;
- as components that run again, with hooks (phase 1).

| Task | Builders | Macro | Hooks |
|---|---|---|---|
| Counter | `samples/SevenGuis/Builders/Counter` | `samples/SevenGuis/Macros/Counter` | `samples/SevenGuis/Hooks/Counter` |
| Temperature converter | `.../Builders/Temperature` | `.../Macros/Temperature` | `.../Hooks/Temperature` |
| Flight booker | `.../Builders/FlightBooker` | `.../Macros/FlightBooker` | `.../Hooks/FlightBooker` |
| Timer | `.../Builders/Timer` | `.../Macros/Timer` | `.../Hooks/Timer` |
| CRUD | `.../Builders/Crud` | `.../Macros/Crud` | `.../Hooks/Crud` |
| Circle drawer | `.../Builders/CircleDrawer` | `.../Macros/CircleDrawer` | `.../Hooks/CircleDrawer` |
| Cells | `.../Builders/Cells` | `.../Macros/Cells` | `.../Hooks/Cells` |

The hooks versions show the model of section 7. Each computes in its function what follows from the state and lets the reconciler patch:
- the flight booker's dates and whether it can book;
- the CRUD's filtered list, as a keyed `for` of `rsx!`;
- the circles, the circle under the pointer and the state of undo and redo.

The timer starts its clock in `use_effect_with` and stops it in the cleanup. The temperature converter writes its fields as controlled inputs: each field shows its state, and a field whose text is already the state changes nothing. Each cell of the spreadsheet runs again on its own; whether it is edited is a `use_memo`, so a change of the edited cell runs only the two cells concerned. The headers are memoized.

`samples/SevenGuis/Model` holds what the three share: the conversions, the dates, the CRUD filter, the circle history, the formulas of the cells (a parser and an evaluator; each cell is a memo of the cells it names, so a change is computed again only in the cells that depend on it, and a cycle is an error until it is broken), the reactive state of the circle drawer and of the cells, and the window a task runs in (in the WinUI 3 theme of `ferroui-themes-winui`). `samples/SevenGuis/Tests` runs one scenario per task on each of the twenty-one crates (what a user types and clicks, and what the task must show), in one test binary.

The windows of the tasks, as each renders itself (`FERROUI_SEVEN_GUIS_SHOTS`). The three versions of a task render the same window. The pictures of the hooks versions are `images/seven-guis/hooks-*.png`:

| Counter | Temperature converter | Flight booker |
|---|---|---|
| ![Counter](images/seven-guis/builders-counter.png) | ![Temperature converter](images/seven-guis/builders-temperature.png) | ![Flight booker](images/seven-guis/builders-flight-booker.png) |

| Timer | CRUD |
|---|---|
| ![Timer](images/seven-guis/builders-timer.png) | ![CRUD](images/seven-guis/builders-crud.png) |

| Circle drawer | Cells |
|---|---|
| ![Circle drawer](images/seven-guis/builders-circle-drawer.png) | ![Cells](images/seven-guis/builders-cells.png) |

```sh
cargo run -p seven-guis-counter-builders
cargo run -p seven-guis-cells-macros
cargo run -p seven-guis-crud-hooks
FERROUI_SEVEN_GUIS_SHOTS=/tmp/shots cargo run -p seven-guis-crud-macros   # renders the window into /tmp/shots/macros-crud.png
```

## 10. Phases

| Phase | Content | State |
|---|---|---|
| 0 | Element tree, generated builders, `rsx!`, `#[component]`, signals, children that follow state, 7GUIs in two versions | done (#270) |
| 1 | Components that run again, hooks, contexts, the reconciler, 7GUIs in a third version | done |
| 2 | Interoperation with XAML and templates | planned |
| 3 | The rest of the API: other crates of controls, styles, resources, animations | planned |
| 4 | Tools: hot reload, an inspector, diagnostics | planned |
| 5 | Performance and scale | planned |
| 6 | Patterns of larger applications | planned |

Each phase lands as one or more pull requests with tests, and samples where they show something new. Each also updates this document and [declarative-ui-react.md](declarative-ui-react.md).

### Phase 2: interoperation with XAML and templates

- **A component in XAML.** A control, `ComponentHost`, holds a component, so that a XAML view can host a part written in Rust. Its content is mounted under the owner of the control and disposed when the control is detached. Properties of the host reach the component as a context or as parameters.
- **XAML in a component.** `Element::Existing` takes any control already. What is missing is to load a XAML resource as an element (`xaml("ferres://..")`), with access to its named controls.
- **Item templates from components.** An `ItemsControl` with virtualization makes its item containers itself, so the reconciler does not drive them. A data template that builds an element per item (`template(|item: &T| rsx! { .. })`) would mount the element in each container and reconcile it when the container is given another item. This makes the declarative API work with virtualized lists, `TreeDataGrid` included.
- **Data context.** A hook that reads the `DataContext` of the control a component is mounted in, for components that live in a view model application (`docs/mvvm.md`).

Acceptance: a XAML window with a Rust component inside, a component with a XAML user control inside, and a virtualized `ListBox` of 10,000 items whose template is a component; each tested.

### Phase 3: the rest of the API

- **Builders for other crates.** The generator runs over `ferroui-controls-treedatagrid`, `ferroui-controls-color-picker`, `ferroui-dialogs` and the ReactiveUI controls, each set behind a feature of `ferroui-view` or in the crate itself.
- **Styles.** A builder for `Style` in the form of the element builders (`style("Button.accent").set(Button::background_property(), ..)`), and styles held by an element (`styles: [..]`).
- **Resources and themes.** `use_resource(key)`, a hook that follows a dynamic resource, and a way to read the theme variant.
- **Animations and transitions.** Builders for `Transitions` and key-frame animations, and a hook that runs an animation when a value changes.
- **Popups.** Elements for popups, flyouts and context menus that are mounted outside their parent's visual tree but under its owner, as React's portals are.

Acceptance: the ControlCatalog page of one control of each crate written with the builders, and a styled sample with a theme switch.

### Phase 4: tools

- **Hot reload of `rsx!`.** The static parts of `rsx!` (the classes, the literal values, the layout) are kept in a form the running application can load again. Only expressions need a rebuild. The reconciler applies the new tree to the mounted controls and keeps their state.
- **An inspector.** The DevTools of FerroUI show the instance tree beside the visual tree: components, their hooks and their keys.
- **Diagnostics.** In debug builds, warnings for siblings of a list without keys, duplicate keys, and a component that runs again more than a given number of times in one propagation of changes.

### Phase 5: performance and scale

- **Moves.** Keyed children reordered with the fewest moves (the longest increasing subsequence of the old positions, as Vue and Inferno do), instead of the prefix and suffix rule.
- **Values that skip work.** Handlers wrapped so that the control keeps one handler and the closure behind it is swapped on each run (no detach and attach). Reactive values compared by the identity of the signal they follow.
- **Measures.** Benchmarks of mounting and of a run of large trees (the cells of the spreadsheet, a list of 10,000 rows), against the run-once model and against a XAML view, in `scripts/` as the TreeDataGrid benchmarks are.
- **Memo by default** for components whose parameters are all `Copy` and `PartialEq`, if the measures show a gain.

### Phase 6: patterns of larger applications

- **Asynchronous data.** `use_resource(future)`, a hook that runs a future on the thread's executor (`ferroui_system::threading::tasks`) and gives its state (pending, done, failed), with a way to show a placeholder while it is pending.
- **Errors.** An error boundary: a component that catches a panic of a child's run and shows a replacement instead of ending the application.
- **Navigation.** A router of views for single-window applications, with the routing of ReactiveUI as a model.
- **A larger sample.** TodoMVC and a port of a ReactiveUI sample to components that run again, in the WinUI 3 theme, with screenshots and scenario tests.

## 11. Testing

- `src/FerroUI.View/tests`: the reactive runtime, setters and conversions, mounting, two-way values, handlers, keyed lists, conditions, the macros and components; and for phase 1 (`rerender_tests.rs`):
  - runs again and kept controls;
  - equal values left alone and missing values cleared;
  - matching by key and by place;
  - handlers attached again, and batching;
  - hooks and child state across a parent's run;
  - memoized components;
  - effects and their cleanups;
  - contexts;
  - the order of hooks;
  - dynamic lists in a component that runs again;
  - `if` and `for` of `rsx!`.
- `samples/SevenGuis/Tests`: the scenarios of the seven tasks on the twenty-one crates, on the test application of `ferroui-controls` (a dispatcher of the thread).
- `samples/SevenGuis/Model`: the logic without user interface.
