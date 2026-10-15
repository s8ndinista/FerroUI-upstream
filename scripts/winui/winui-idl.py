#!/usr/bin/env python3
"""Writes src/Windows/FerroUI.WinUI/interop/winui.idl, the interface definition file of the WinUI
integration, from the metadata of WinUI 3 and of the Windows SDK (docs/porting/winui.md, section 3).

    scripts/winui/winui-idl.py --xaml <Microsoft.UI.Xaml.winmd> --ui <Microsoft.UI.winmd> \
        --universal <Windows.Foundation.UniversalApiContract.winmd> \
        --foundation <Windows.Foundation.FoundationContract.winmd> --out <winui.idl>

The metadata files are those of the package Microsoft.WindowsAppSDK 1.6.241114003 (lib/uap10.0 and
lib/uap10.0.18362 of the package) and of Microsoft.Windows.SDK.NET.Ref 10.0.19041.57 (winmd/), both
on nuget.org. They are read through the text the IL disassembler `ikdasm` of Mono prints of them
(on the PATH, or `--ikdasm <path>`): every interface with its identifier and its methods in the
order of its vtable, the delegates, enumerations, structures and runtime classes.

The file declares the interfaces of SPEC below, in that order, the enumerations of ENUMS and every
structure they pass. A parameterized instance gets the identifier the Windows Runtime derives from
its signature, which is written in the comment above it. Standard library only.
"""
import argparse
import hashlib
import re
import subprocess
import sys
import uuid

# The declarations of the metadata


def attr_bytes(lines, i):
    """The bytes of a custom attribute blob that starts at line i."""
    j = i
    started = False
    out = []
    while True:
        code = lines[j].split("//")[0]
        if not started:
            if "= (" in code:
                started = True
                code = code.split("= (", 1)[1]
            else:
                j += 1
                continue
        done = ")" in code
        code = code.replace(")", "")
        out += [int(b, 16) for b in code.split() if re.fullmatch(r"[0-9A-F]{2}", b)]
        if done:
            return out, j
        j += 1


def guid_from_blob(b):
    import struct
    d1, d2, d3 = struct.unpack("<IHH", bytes(b[2:10]))
    rest = b[10:18]
    return "%08X-%04X-%04X-%02X%02X-%s" % (d1, d2, d3, rest[0], rest[1], "".join("%02X" % x for x in rest[2:]))


def ser_string(b, pos):
    n = b[pos]
    pos += 1
    if n == 0xFF:
        return None, pos
    if n & 0x80:
        n = ((n & 0x3F) << 8) | b[pos]
        pos += 1
    return bytes(b[pos:pos + n]).decode(), pos + n


class Method:
    def __init__(self, name, ret, params, static, overload):
        self.name, self.ret, self.params, self.static, self.overload = name, ret, params, static, overload


class Type:
    def __init__(self, name, kind):
        self.name = name
        self.kind = kind
        self.guid = None
        self.methods = []
        self.fields = []
        self.extends = None
        self.implements = []
        self.default = None
        self.factories = []  # (kind, interface)
        self.underlying = "int32"


def split_params(s):
    out, depth, cur = [], 0, ""
    for ch in s:
        if ch in "<":
            depth += 1
        elif ch in ">":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur)
    return [p.strip() for p in out]


ASM = re.compile(r"\[[A-Za-z0-9_.]+\]")


def clean_type(t):
    t = ASM.sub("", t).strip()
    t = re.sub(r"^(class|valuetype)\s+", "", t)
    t = re.sub(r"<(class|valuetype)\s+", "<", t)
    t = re.sub(r",(class|valuetype)\s+", ",", t)
    return t.strip()


def parse_text(text):
    lines = text.split("\n")
    types = {}
    i = 0
    n = len(lines)
    while i < n:
        line = lines[i]
        if line.startswith(".class "):
            header = line
            j = i + 1
            while not lines[j].startswith("{"):
                header += " " + lines[j].strip()
                j += 1
            m = re.search(r"windowsruntime\s+(?:sealed\s+)?(?:abstract\s+)?([A-Za-z0-9_.`]+)", header)
            name = m.group(1) if m else header.split()[-1]
            if " interface " in header:
                kind = "interface"
            elif "System.MulticastDelegate" in header:
                kind = "delegate"
            elif "System.Enum" in header:
                kind = "enum"
            elif "System.ValueType" in header:
                kind = "struct"
            else:
                kind = "class"
            t = Type(name, kind)
            em = re.search(r"extends\s+(\S+)", header)
            if em:
                t.extends = clean_type(em.group(1))
            types[name] = t
            # body
            k = j + 1
            pending_iface = None
            while not lines[k].startswith("} // end of class"):
                l = lines[k]
                s = l.strip()
                if s.startswith(".custom") and "GuidAttribute" in s and t.guid is None and k < len(lines):
                    b, k = attr_bytes(lines, k)
                    t.guid = guid_from_blob(b)
                elif s.startswith(".custom") and ("ComposableAttribute" in s or "StaticAttribute" in s or "ActivatableAttribute" in s) and not l.startswith("    "):
                    which = "composable" if "Composable" in s else ("static" if "StaticAttribute" in s else "activatable")
                    b, k = attr_bytes(lines, k)
                    if b[2] == 0xFF or which == "activatable" and len(b) < 12:
                        pass
                    try:
                        sname, _ = ser_string(b, 2)
                        if sname and "." in sname:
                            t.factories.append((which, sname))
                        elif which == "activatable":
                            t.factories.append((which, None))
                    except Exception:
                        pass
                elif s.startswith(".interfaceimpl type"):
                    pending_iface = clean_type(s[len(".interfaceimpl type"):])
                    t.implements.append(pending_iface)
                elif s.startswith(".custom") and "DefaultAttribute" in s and pending_iface:
                    t.default = pending_iface
                    pending_iface = None
                elif s.startswith(".field") and kind in ("enum", "struct"):
                    if kind == "enum":
                        fm = re.search(r"literal valuetype \S+ (\S+) = (u?int\d+)\((0x[0-9A-Fa-f]+)\)", s)
                        if fm:
                            t.fields.append((fm.group(1), int(fm.group(3), 16), fm.group(2)))
                        um = re.search(r"specialname rtspecialname (\S+) value__", s)
                        if um:
                            t.underlying = um.group(1)
                    else:
                        fm = re.search(r"\.field public (.+) (\S+)$", s)
                        if fm:
                            t.fields.append((fm.group(2).strip("'"), clean_type(fm.group(1))))
                elif s.startswith(".method"):
                    head = s
                    k += 1
                    while "cil managed" not in head and "runtime managed" not in head:
                        head += " " + lines[k].strip()
                        k += 1
                    static = " static " in head
                    body_overload = None
                    # body until "} // end of method"
                    while not lines[k].strip().startswith("} // end of method"):
                        if "OverloadAttribute" in lines[k]:
                            b, k2 = attr_bytes(lines, k)
                            body_overload, _ = ser_string(b, 2)
                            k = k2
                        k += 1
                    mm = re.search(r"instance (.+?)\s+([A-Za-z0-9_.]+)\((.*)\) (cil|runtime) managed", head) or \
                        re.search(r"static (.+?)\s+([A-Za-z0-9_.]+)\((.*)\) (cil|runtime) managed", head)
                    if mm:
                        ret = clean_type(mm.group(1).replace("hidebysig", "").replace("specialname", "").replace("rtspecialname", "").replace("newslot", "").replace("abstract", "").replace("virtual", "").replace("final", "").replace("public", ""))
                        params = []
                        for p in split_params(mm.group(3)):
                            pm = re.match(r"(\[in\]|\[out\])?\s*(.+)\s+'?([A-Za-z0-9_]+)'?$", p)
                            if pm:
                                params.append((pm.group(1) or "", clean_type(pm.group(2)), pm.group(3)))
                        t.methods.append(Method(mm.group(2), ret, params, static, body_overload))
                k += 1
            i = k
        i += 1
    return types


# The IDL

T = {}

PRIM = {
    "bool": ("boolean", "b1"), "char": ("WCHAR", "c2"), "uint8": ("BYTE", "u1"), "int16": ("INT16", "i2"),
    "uint16": ("UINT16", "u2"), "int32": ("INT32", "i4"), "uint32": ("UINT32", "u4"), "int64": ("INT64", "i8"),
    "uint64": ("UINT64", "u8"), "float32": ("FLOAT", "f4"), "float64": ("DOUBLE", "f8"), "string": ("HSTRING", "string"),
    "object": ("IInspectable*", "cinterface(IInspectable)"), "System.Guid": ("GUID", "g16"), "Guid": ("GUID", "g16"),
}

# Generic instances: IDL name -> metadata spelling.
GENERICS = {}


def short(name):
    return name.split(".")[-1].replace("`", "_")


def split_generic(t):
    m = re.match(r"^([A-Za-z0-9_.]+`\d+)<(.*)>$", t)
    if not m:
        return None
    return m.group(1), split_params(m.group(2))


def signature(t):
    if t in PRIM:
        return PRIM[t][1]
    g = split_generic(t)
    if g:
        base, args = g
        return "pinterface({%s};%s)" % (T[base].guid.lower(), ";".join(signature(a) for a in args))
    ty = T[t]
    if ty.kind == "interface":
        return "{%s}" % ty.guid.lower()
    if ty.kind == "delegate":
        return "delegate({%s})" % ty.guid.lower()
    if ty.kind == "enum":
        flags = ty.underlying == "uint32"
        return "enum(%s;%s)" % (t, "u4" if flags else "i4")
    if ty.kind == "struct":
        return "struct(%s;%s)" % (t, ";".join(signature(f[1]) for f in ty.fields))
    if ty.kind == "class":
        return "rc(%s;%s)" % (t, signature(ty.default))
    raise Exception(t)


NS = uuid.UUID("11f47ad5-7b73-42c0-abae-878b1e16adee")


def piid(t):
    sig = signature(t)
    h = hashlib.sha1(NS.bytes + sig.encode()).digest()
    b = bytearray(h[:16])
    b[6] = (b[6] & 0x0F) | 0x50
    b[8] = (b[8] & 0x3F) | 0x80
    return str(uuid.UUID(bytes=bytes(b))).upper(), sig


declared = {}  # metadata name -> idl name
structs_needed = []


def idl_name_of(t):
    for k, v in GENERICS.items():
        if v == t:
            return k
    return declared.get(t)


def map_type(t):
    """IDL type of a value of metadata type t (as passed in)."""
    t = t.strip()
    if t in PRIM:
        return PRIM[t][0]
    if t == "Windows.Foundation.HResult":
        return "INT32"
    n = idl_name_of(t)
    if n:
        return n + "*"
    if split_generic(t):
        return "void*"
    if t.startswith("!"):
        raise Exception("open generic " + t)
    ty = T.get(t)
    if ty is None:
        raise Exception("unknown " + t)
    if ty.kind == "enum":
        if short(t) in ENUMS:
            return short(t)
        return "UINT32" if ty.underlying == "uint32" else "INT32"
    if ty.kind == "struct":
        if t not in structs_needed:
            for f in ty.fields:
                map_type(f[1])
            structs_needed.append(t)
        return short(t)
    if ty.kind == "class":
        d = idl_name_of(ty.default) if ty.default else None
        return (d + "*") if d else "IInspectable*"
    return "void*"


def emit_method(m, subst):
    def s(x):
        for k, v in subst.items():
            x = x.replace(k, v)
        return x
    params = []
    for (direction, ty, name) in m.params:
        ty = s(ty)
        if ty.endswith("[]&"):
            base = ty[:-3]
            params.append("[out] UINT32* %sSize" % name)
            params.append("[out] %s** %s" % (map_type(base), name))
        elif ty.endswith("[]"):
            base = ty[:-2]
            params.append("UINT32 %sSize" % name)
            params.append("%s* %s" % (map_type(base), name))
        elif ty.endswith("&"):
            params.append("[out] %s* %s" % (map_type(ty[:-1]), name))
        else:
            params.append("%s %s" % (map_type(ty), name))
    ret = s(m.ret)
    if ret.endswith("[]"):
        params.append("[out] UINT32* resultSize")
        params.append("[out] %s** result" % map_type(ret[:-2]))
    elif ret != "void":
        params.append("[out, retval] %s* result" % map_type(ret))
    name = m.overload or m.name
    attr = ""
    if name.startswith("get_") and m.ret != "void" and not m.params:
        attr, name = "[propget] ", name[4:]
    elif name.startswith("put_") and m.ret == "void" and len(m.params) == 1:
        attr, name = "[propput] ", name[4:]
    return "    %sHRESULT %s(%s);" % (attr, name, ", ".join(params))


def emit_interface(idl, t, comment=None, base="IInspectable"):
    g = split_generic(t)
    if g:
        gbase, args = g
        ty = T[gbase]
        gen_params = {"IVector`1": ["T"], "IVectorView`1": ["T"], "IIterable`1": ["T"], "IIterator`1": ["T"],
                      "TypedEventHandler`2": ["TSender", "TResult"], "IAsyncOperation`1": ["TResult"],
                      "AsyncOperationCompletedHandler`1": ["TResult"], "IReference`1": ["T"], "EventHandler`1": ["T"]}[short(gbase).replace("_", "`")]
        subst = {"!" + p: a for p, a in zip(gen_params, args)}
        guid, sig = piid(t)
        out = ["// %s: %s" % (t, sig)]
    else:
        ty = T[t]
        subst = {}
        guid = ty.guid
        out = ["// %s" % t] if comment is None else ["// " + comment]
    kind_base = "IUnknown" if ty.kind == "delegate" else base
    methods = [m for m in ty.methods if m.name not in (".ctor",) and not (ty.kind == "delegate" and m.name != "Invoke")]
    out.append("[uuid(%s)]" % guid)
    out.append("interface %s : %s" % (idl, kind_base))
    out.append("{")
    for m in methods:
        out.append(emit_method(m, subst))
    out.append("}")
    return "\n".join(out)


def struct_decl(t):
    ty = T[t]
    lines = ["// %s" % t, "struct %s" % short(t), "{"]
    for (n, ft) in ty.fields:
        lines.append("    %s %s;" % (map_type(ft), n))
    lines.append("}")
    return "\n".join(lines)


def enum_decl(t):
    ty = T[t]
    lines = ["// %s (%s)" % (t, ty.underlying), "enum %s" % short(t), "{"]
    for (n, v, _) in ty.fields:
        lines.append("    %s = %d," % (n, v))
    lines.append("}")
    return "\n".join(lines)


def run(spec):
    """spec: list of (idl_name, metadata_name)."""
    for idl, md in spec:
        if split_generic(md):
            GENERICS[idl] = md
        else:
            declared[md] = idl
    bodies = []
    for idl, md in spec:
        bodies.append(emit_interface(idl, md))
    return bodies


def find_enum(shortname):
    found = [k for k, v in T.items() if v.kind == "enum" and k.split(".")[-1] == shortname]
    for prefix in ("Microsoft.UI.", "Windows.System.", "Windows.UI.Core.", "Windows.UI.Text.Core.", "Windows.ApplicationModel.DataTransfer.", "Windows.Foundation."):
        for k in found:
            if k.startswith(prefix):
                return k
    raise Exception(shortname + str(found))


# What is declared

ENUMS = []

X = "Microsoft.UI.Xaml."
F = "Windows.Foundation."
C = "Windows.Foundation.Collections."

ENUMS += [
    "AutomationControlType", "PatternInterface", "AutomationEvents", "ToggleState", "ExpandCollapseState",
    "ScrollAmount", "FocusState", "FocusNavigationDirection", "PointerDeviceType", "PointerUpdateKind",
    "InputSystemCursorShape", "VirtualKey", "VirtualKeyModifiers", "CoreVirtualKeyStates",
    "CoreTextInputScope", "CoreTextInputPaneDisplayPolicy", "CoreTextTextUpdatingResult",
    "CoreTextSelectionUpdatingResult", "CoreTextFormatUpdatingResult", "DataPackageOperation", "AsyncStatus",
]

SPEC = [
    # Foundation
    ("IAsyncInfo", F + "IAsyncInfo"),
    ("IClosable", F + "IClosable"),
    # Microsoft.UI.Xaml: the application and the window
    ("IApplication", X + "IApplication"),
    ("IApplicationStatics", X + "IApplicationStatics"),
    ("IApplicationFactory", X + "IApplicationFactory"),
    ("IApplicationOverrides", X + "IApplicationOverrides"),
    ("ApplicationInitializationCallback", X + "ApplicationInitializationCallback"),
    ("IWindow", X + "IWindow"),
    ("IWindowFactory", X + "IWindowFactory"),
    ("IWindowStatics", X + "IWindowStatics"),
    # The resources of the application (XamlControlsResources, merged as the App.xaml of the
    # reference's samples merges it) and the type information of the controls library
    ("IResourceDictionary", X + "IResourceDictionary"),
    ("IResourceDictionaryFactory", X + "IResourceDictionaryFactory"),
    ("IXamlMetadataProvider", X + "Markup.IXamlMetadataProvider"),
    ("IXamlControlsXamlMetaDataProviderStatics", X + "XamlTypeInfo.IXamlControlsXamlMetaDataProviderStatics"),
    ("IXamlReaderStatics", X + "Markup.IXamlReaderStatics"),
    # Elements
    ("IDependencyObject", X + "IDependencyObject"),
    ("IUIElement", X + "IUIElement"),
    ("IUIElementOverrides", X + "IUIElementOverrides"),
    ("IUIElementProtected", X + "IUIElementProtected"),
    ("IUIElementStatics", X + "IUIElementStatics"),
    ("IFrameworkElement", X + "IFrameworkElement"),
    ("IPanel", X + "Controls.IPanel"),
    ("IGridFactory", X + "Controls.IGridFactory"),
    ("IGridStatics", X + "Controls.IGridStatics"),
    # The controls of WinUIEmbedSample
    ("ITextBlock", X + "Controls.ITextBlock"),
    ("ITextBox", X + "Controls.ITextBox"),
    ("IButtonBase", X + "Controls.Primitives.IButtonBase"),
    ("IRangeBase", X + "Controls.Primitives.IRangeBase"),
    ("IRangeBaseValueChangedEventArgs", X + "Controls.Primitives.IRangeBaseValueChangedEventArgs"),
    ("RangeBaseValueChangedEventHandler", X + "Controls.Primitives.RangeBaseValueChangedEventHandler"),
    ("ISelector", X + "Controls.Primitives.ISelector"),
    ("ISelectionChangedEventArgs", X + "Controls.ISelectionChangedEventArgs"),
    ("SelectionChangedEventHandler", X + "Controls.SelectionChangedEventHandler"),
    ("ISwapChainPanel", X + "Controls.ISwapChainPanel"),
    ("ISwapChainPanelFactory", X + "Controls.ISwapChainPanelFactory"),
    ("IBrush", X + "Media.IBrush"),
    ("ISolidColorBrushFactory", X + "Media.ISolidColorBrushFactory"),
    ("IGeneralTransform", X + "Media.IGeneralTransform"),
    ("IXamlRoot", X + "IXamlRoot"),
    ("IXamlRoot2", X + "IXamlRoot2"),
    ("IWindowEventArgs", X + "IWindowEventArgs"),
    ("IRoutedEvent", X + "IRoutedEvent"),
    ("IRoutedEventArgs", X + "IRoutedEventArgs"),
    ("ISizeChangedEventArgs", X + "ISizeChangedEventArgs"),
    ("RoutedEventHandler", X + "RoutedEventHandler"),
    ("SizeChangedEventHandler", X + "SizeChangedEventHandler"),
    ("DragEventHandler", X + "DragEventHandler"),
    ("IDragEventArgs", X + "IDragEventArgs"),
    ("IDragOperationDeferral", X + "IDragOperationDeferral"),
    ("IDragStartingEventArgs", X + "IDragStartingEventArgs"),
    ("IDragUI", X + "IDragUI"),
    # Input
    ("IPointer", X + "Input.IPointer"),
    ("IPointerRoutedEventArgs", X + "Input.IPointerRoutedEventArgs"),
    ("PointerEventHandler", X + "Input.PointerEventHandler"),
    ("IKeyRoutedEventArgs", X + "Input.IKeyRoutedEventArgs"),
    ("KeyEventHandler", X + "Input.KeyEventHandler"),
    ("ICharacterReceivedRoutedEventArgs", X + "Input.ICharacterReceivedRoutedEventArgs"),
    ("IFocusManagerStatics", X + "Input.IFocusManagerStatics"),
    ("IFindNextElementOptions", X + "Input.IFindNextElementOptions"),
    # Automation
    ("IAutomationPeer", X + "Automation.Peers.IAutomationPeer"),
    ("IAutomationPeerOverrides", X + "Automation.Peers.IAutomationPeerOverrides"),
    ("IAutomationPeerProtected", X + "Automation.Peers.IAutomationPeerProtected"),
    ("IAutomationPeerStatics", X + "Automation.Peers.IAutomationPeerStatics"),
    ("IAutomationPeerFactory", X + "Automation.Peers.IAutomationPeerFactory"),
    ("IFrameworkElementAutomationPeer", X + "Automation.Peers.IFrameworkElementAutomationPeer"),
    ("IFrameworkElementAutomationPeerFactory", X + "Automation.Peers.IFrameworkElementAutomationPeerFactory"),
    ("IFrameworkElementAutomationPeerStatics", X + "Automation.Peers.IFrameworkElementAutomationPeerStatics"),
    ("IIRawElementProviderSimple", X + "Automation.Provider.IIRawElementProviderSimple"),
    ("IInvokeProvider", X + "Automation.Provider.IInvokeProvider"),
    ("IToggleProvider", X + "Automation.Provider.IToggleProvider"),
    ("IValueProvider", X + "Automation.Provider.IValueProvider"),
    ("IRangeValueProvider", X + "Automation.Provider.IRangeValueProvider"),
    ("IScrollProvider", X + "Automation.Provider.IScrollProvider"),
    ("IScrollItemProvider", X + "Automation.Provider.IScrollItemProvider"),
    ("ISelectionProvider", X + "Automation.Provider.ISelectionProvider"),
    ("ISelectionItemProvider", X + "Automation.Provider.ISelectionItemProvider"),
    ("IExpandCollapseProvider", X + "Automation.Provider.IExpandCollapseProvider"),
    # Microsoft.UI
    ("IInputCursor", "Microsoft.UI.Input.IInputCursor"),
    ("IInputSystemCursor", "Microsoft.UI.Input.IInputSystemCursor"),
    ("IInputSystemCursorStatics", "Microsoft.UI.Input.IInputSystemCursorStatics"),
    ("IPointerPoint", "Microsoft.UI.Input.IPointerPoint"),
    ("IPointerPointProperties", "Microsoft.UI.Input.IPointerPointProperties"),
    ("IInputKeyboardSourceStatics", "Microsoft.UI.Input.IInputKeyboardSourceStatics"),
    ("IDispatcherQueue", "Microsoft.UI.Dispatching.IDispatcherQueue"),
    ("DispatcherQueueHandler", "Microsoft.UI.Dispatching.DispatcherQueueHandler"),
    ("IContentIslandEnvironment", "Microsoft.UI.Content.IContentIslandEnvironment"),
    # Windows: text input
    ("ICoreTextServicesManagerStatics", "Windows.UI.Text.Core.ICoreTextServicesManagerStatics"),
    ("ICoreTextServicesManager", "Windows.UI.Text.Core.ICoreTextServicesManager"),
    ("ICoreTextEditContext", "Windows.UI.Text.Core.ICoreTextEditContext"),
    ("ICoreTextTextRequestedEventArgs", "Windows.UI.Text.Core.ICoreTextTextRequestedEventArgs"),
    ("ICoreTextTextRequest", "Windows.UI.Text.Core.ICoreTextTextRequest"),
    ("ICoreTextSelectionRequestedEventArgs", "Windows.UI.Text.Core.ICoreTextSelectionRequestedEventArgs"),
    ("ICoreTextSelectionRequest", "Windows.UI.Text.Core.ICoreTextSelectionRequest"),
    ("ICoreTextLayoutRequestedEventArgs", "Windows.UI.Text.Core.ICoreTextLayoutRequestedEventArgs"),
    ("ICoreTextLayoutRequest", "Windows.UI.Text.Core.ICoreTextLayoutRequest"),
    ("ICoreTextLayoutBounds", "Windows.UI.Text.Core.ICoreTextLayoutBounds"),
    ("ICoreTextTextUpdatingEventArgs", "Windows.UI.Text.Core.ICoreTextTextUpdatingEventArgs"),
    ("ICoreTextSelectionUpdatingEventArgs", "Windows.UI.Text.Core.ICoreTextSelectionUpdatingEventArgs"),
    ("ICoreTextFormatUpdatingEventArgs", "Windows.UI.Text.Core.ICoreTextFormatUpdatingEventArgs"),
    ("ICoreTextCompositionStartedEventArgs", "Windows.UI.Text.Core.ICoreTextCompositionStartedEventArgs"),
    ("ICoreTextCompositionCompletedEventArgs", "Windows.UI.Text.Core.ICoreTextCompositionCompletedEventArgs"),
    # Windows: data transfer and storage
    ("IDataPackageView", "Windows.ApplicationModel.DataTransfer.IDataPackageView"),
    ("IDataPackage", "Windows.ApplicationModel.DataTransfer.IDataPackage"),
    ("IStandardDataFormatsStatics", "Windows.ApplicationModel.DataTransfer.IStandardDataFormatsStatics"),
    ("IStorageItem", "Windows.Storage.IStorageItem"),
    ("IStorageFile", "Windows.Storage.IStorageFile"),
    ("IStorageFolder", "Windows.Storage.IStorageFolder"),
    ("IStorageFileStatics", "Windows.Storage.IStorageFileStatics"),
    ("IStorageFolderStatics", "Windows.Storage.IStorageFolderStatics"),
    # Generic instances
    ("IVectorUIElement", C + "IVector`1<Microsoft.UI.Xaml.UIElement>"),
    ("IVectorResourceDictionary", C + "IVector`1<Microsoft.UI.Xaml.ResourceDictionary>"),
    ("IVectorAutomationPeer", C + "IVector`1<Microsoft.UI.Xaml.Automation.Peers.AutomationPeer>"),
    ("IVectorViewAutomationPeer", C + "IVectorView`1<Microsoft.UI.Xaml.Automation.Peers.AutomationPeer>"),
    ("IIterableAutomationPeer", C + "IIterable`1<Microsoft.UI.Xaml.Automation.Peers.AutomationPeer>"),
    ("IIteratorAutomationPeer", C + "IIterator`1<Microsoft.UI.Xaml.Automation.Peers.AutomationPeer>"),
    ("IVectorPointerPoint", C + "IVector`1<Microsoft.UI.Input.PointerPoint>"),
    ("IVectorViewStorageItem", C + "IVectorView`1<Windows.Storage.IStorageItem>"),
    ("IIterableStorageItem", C + "IIterable`1<Windows.Storage.IStorageItem>"),
    ("IIteratorStorageItem", C + "IIterator`1<Windows.Storage.IStorageItem>"),
    ("IReferenceKeyEventHandler", F + "IReference`1<Microsoft.UI.Xaml.Input.KeyEventHandler>"),
    ("IReferenceCharacterReceivedHandler", F + "IReference`1<Windows.Foundation.TypedEventHandler`2<Microsoft.UI.Xaml.UIElement,Microsoft.UI.Xaml.Input.CharacterReceivedRoutedEventArgs>>"),
    ("TypedEventHandlerObjectWindowEventArgs", F + "TypedEventHandler`2<object,Microsoft.UI.Xaml.WindowEventArgs>"),
    ("TypedEventHandlerSwapChainPanelObject", F + "TypedEventHandler`2<Microsoft.UI.Xaml.Controls.SwapChainPanel,object>"),
    ("TypedEventHandlerUIElementCharacterReceived", F + "TypedEventHandler`2<Microsoft.UI.Xaml.UIElement,Microsoft.UI.Xaml.Input.CharacterReceivedRoutedEventArgs>"),
    ("TypedEventHandlerUIElementDragStarting", F + "TypedEventHandler`2<Microsoft.UI.Xaml.UIElement,Microsoft.UI.Xaml.DragStartingEventArgs>"),
    ("TypedEventHandlerFrameworkElementObject", F + "TypedEventHandler`2<Microsoft.UI.Xaml.FrameworkElement,object>"),
    ("TypedEventHandlerTextRequested", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,Windows.UI.Text.Core.CoreTextTextRequestedEventArgs>"),
    ("TypedEventHandlerSelectionRequested", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,Windows.UI.Text.Core.CoreTextSelectionRequestedEventArgs>"),
    ("TypedEventHandlerLayoutRequested", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,Windows.UI.Text.Core.CoreTextLayoutRequestedEventArgs>"),
    ("TypedEventHandlerTextUpdating", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,Windows.UI.Text.Core.CoreTextTextUpdatingEventArgs>"),
    ("TypedEventHandlerSelectionUpdating", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,Windows.UI.Text.Core.CoreTextSelectionUpdatingEventArgs>"),
    ("TypedEventHandlerFormatUpdating", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,Windows.UI.Text.Core.CoreTextFormatUpdatingEventArgs>"),
    ("TypedEventHandlerCompositionStarted", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,Windows.UI.Text.Core.CoreTextCompositionStartedEventArgs>"),
    ("TypedEventHandlerCompositionCompleted", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,Windows.UI.Text.Core.CoreTextCompositionCompletedEventArgs>"),
    ("TypedEventHandlerFocusRemoved", F + "TypedEventHandler`2<Windows.UI.Text.Core.CoreTextEditContext,object>"),
    ("IAsyncOperationDataPackageOperation", F + "IAsyncOperation`1<Windows.ApplicationModel.DataTransfer.DataPackageOperation>"),
    ("AsyncOperationCompletedHandlerDataPackageOperation", F + "AsyncOperationCompletedHandler`1<Windows.ApplicationModel.DataTransfer.DataPackageOperation>"),
    ("IAsyncOperationStorageItems", F + "IAsyncOperation`1<Windows.Foundation.Collections.IVectorView`1<Windows.Storage.IStorageItem>>"),
    ("AsyncOperationCompletedHandlerStorageItems", F + "AsyncOperationCompletedHandler`1<Windows.Foundation.Collections.IVectorView`1<Windows.Storage.IStorageItem>>"),
    ("IAsyncOperationString", F + "IAsyncOperation`1<string>"),
    ("AsyncOperationCompletedHandlerString", F + "AsyncOperationCompletedHandler`1<string>"),
    ("IAsyncOperationStorageFile", F + "IAsyncOperation`1<Windows.Storage.StorageFile>"),
    ("AsyncOperationCompletedHandlerStorageFile", F + "AsyncOperationCompletedHandler`1<Windows.Storage.StorageFile>"),
    ("IAsyncOperationStorageFolder", F + "IAsyncOperation`1<Windows.Storage.StorageFolder>"),
    ("AsyncOperationCompletedHandlerStorageFolder", F + "AsyncOperationCompletedHandler`1<Windows.Storage.StorageFolder>"),
]


HEAD = '''// The COM interfaces of the Windows Runtime the WinUI integration uses, for the MicroCom
// generator (src/tools/MicroCom.CodeGenerator), as the Windows backend declares its own in
// `win_rt/winrt.idl`. The reference has no such file: it is written against the projections
// of its runtime (C#/WinRT). Every interface below is the one of the metadata, with its
// identifier and its methods in the order of the vtable:
//
// * Microsoft.UI.Xaml, Microsoft.UI.Input, Microsoft.UI.Dispatching, Microsoft.UI.Content:
//   `Microsoft.UI.Xaml.winmd` and `Microsoft.UI.winmd` of the Windows App SDK 1.6
//   (package Microsoft.WindowsAppSDK 1.6.241114003);
// * Windows.*: `Windows.Foundation.UniversalApiContract.winmd` and
//   `Windows.Foundation.FoundationContract.winmd` of the Windows SDK 10.0.19041.
//
// `scripts/winui/winui-idl.py` writes this file from those files (docs/porting/winui.md,
// section 3). A property is a `[propget]` and a `[propput]` method, an event the two methods
// `add_` and `remove_`; an overloaded method has the name of its overload attribute. A
// runtime class is passed as its default interface, a class or an interface the file does
// not declare as `IInspectable*` or `void*`, an enumeration the file does not declare as its
// underlying type. A parameterized interface or delegate is declared for each instance the
// integration uses, under a name of its own, with the identifier the Windows Runtime
// derives from its signature (the comment above it; a test of the crate derives it again).
@rust-types-from super
@rust-map HSTRING isize
@rust-map GUID Guid
@rust-map boolean u8
@rust-map BYTE u8
@rust-map WCHAR u16
@rust-map INT16 i16
@rust-map UINT16 u16
@rust-map INT32 i32
@rust-map UINT32 u32
@rust-map INT64 i64
@rust-map UINT64 u64
@rust-map FLOAT f32
@rust-map DOUBLE f64

enum TrustLevel
{
    BaseTrust,
    PartialTrust,
    FullTrust
}

[uuid(AF86E2E0-B12D-4c6a-9C5A-D7AA65101E90)]
interface IInspectable : IUnknown
{
    HRESULT GetIids([out] UINT32* iidCount, [out] GUID** iids);
    HRESULT GetRuntimeClassName([out] HSTRING* className);
    HRESULT GetTrustLevel([out] TrustLevel* trustLevel);
}

[uuid(00000035-0000-0000-C000-000000000046)]
interface IActivationFactory : IInspectable
{
    HRESULT ActivateInstance([out, retval] IInspectable** instance);
}

// objidl.h: an object that may be called on any thread without marshaling. The handlers of the
// completion of an asynchronous operation answer to it (the operation calls them on the thread
// it completes on, as it calls the handlers of C#/WinRT, which are agile too).
[uuid(94ea2b94-e9cc-49e0-c0ff-ee64ca8f5b90)]
interface IAgileObject : IUnknown
{
}

// microsoft.ui.xaml.media.dxinterop.h of the Windows App SDK: how a swap chain is given to a
// swap chain panel.
[uuid(63aad0b8-7c24-40ff-85a8-640d944cc325)]
interface ISwapChainPanelNative : IUnknown
{
    HRESULT SetSwapChain(void* swapChain);
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--xaml", required=True)
    parser.add_argument("--ui", required=True)
    parser.add_argument("--universal", required=True)
    parser.add_argument("--foundation", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--ikdasm", default="ikdasm")
    arguments = parser.parse_args()

    for path in (arguments.xaml, arguments.ui, arguments.universal, arguments.foundation):
        text = subprocess.run([arguments.ikdasm, path], check=True, stdout=subprocess.PIPE).stdout.decode("utf-8", "replace")
        T.update(parse_text(text))

    bodies = run(SPEC)
    structs = [struct_decl(s) for s in structs_needed]
    enums = [enum_decl(find_enum(e)) for e in ENUMS]
    out = HEAD + "\n// Enumerations\n\n" + "\n".join(enums) + "\n\n// Structures\n\n" + "\n".join(structs) \
        + "\n\n// Interfaces and delegates\n\n" + "\n\n".join(bodies) + "\n"
    with open(arguments.out, "w", encoding="utf-8") as f:
        f.write(out)


if __name__ == "__main__":
    main()
