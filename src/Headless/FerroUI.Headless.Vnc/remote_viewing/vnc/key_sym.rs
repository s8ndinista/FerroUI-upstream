//! Port of `Vnc/KeySym.cs` of RemoteViewing (BSD-2-Clause, Copyright (c)
//! 2013 James F. Bellinger; `NOTICE.md` of this crate).
//!
//! The members are in the order of the original, with its names and values
//! (generated from it). Several names stand for one value and a key event
//! may carry any value, so the symbols are constants of a transparent
//! wrapper of the value and not the variants of an enum (as the key symbols
//! of the X11 platform are).

/// The X11 key symbol of a key, as a VNC client sends it. For many keys
/// this is the ASCII value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct KeySym(pub u32);

#[allow(non_upper_case_globals)]
impl KeySym {
    /// The void symbol.
    pub const VoidSymbol: KeySym = KeySym(0xffffff);
    // TTY function keys, cleverly chosen to map to ASCII, for convenience of
    // programming, but could have been arbitrary (at the cost of lookup
    // tables in client code).
    /// Back space key
    pub const Backspace: KeySym = KeySym(0xff08);
    /// The tab key
    pub const Tab: KeySym = KeySym(0xff09);
    /// The line feed key
    pub const LineFeed: KeySym = KeySym(0xff0a); // Linefeed, LF
    /// The clear key.
    pub const Clear: KeySym = KeySym(0xff0b);
    /// The return (enter) key.
    pub const Return: KeySym = KeySym(0xff0d); // Return, enter
    /// The pause key
    pub const Pause: KeySym = KeySym(0xff13); // Pause, hold
    /// The scroll lock key.
    pub const ScrollLock: KeySym = KeySym(0xff14);
    /// The Sys Req key.
    pub const SysReq: KeySym = KeySym(0xff15);
    /// The escape key.
    pub const Escape: KeySym = KeySym(0xff1b);
    /// The delete key.
    pub const Delete: KeySym = KeySym(0xffff); // Delete, rubout
                                               // Cursor control & motion
    /// The home key
    pub const Home: KeySym = KeySym(0xff50);
    /// The left arrow key.
    pub const Left: KeySym = KeySym(0xff51); // Move left, left arrow
    /// The up arrow key.
    pub const Up: KeySym = KeySym(0xff52); // Move up, up arrow
    /// The right arrow key.
    pub const Right: KeySym = KeySym(0xff53); // Move right, right arrow
    /// The down key.
    pub const Down: KeySym = KeySym(0xff54); // Move down, down arrow
    /// The previous key.
    pub const Prior: KeySym = KeySym(0xff55); // Prior, previous
    /// The page up key.
    pub const PageUp: KeySym = KeySym(0xff55);
    /// The next key.
    pub const Next: KeySym = KeySym(0xff56); // Next
    /// The page down key.
    pub const PageDown: KeySym = KeySym(0xff56);
    /// The end key.
    pub const End: KeySym = KeySym(0xff57); // EOL
    /// The begin key.
    pub const Begin: KeySym = KeySym(0xff58); // BOL
                                              // Misc functions
    /// The select key.
    pub const Select: KeySym = KeySym(0xff60); // Select, mark
    /// The print screen key.
    pub const Print: KeySym = KeySym(0xff61);
    /// The execute key.
    pub const Execute: KeySym = KeySym(0xff62); // Execute, run, do
    /// The insert key.
    pub const Insert: KeySym = KeySym(0xff63); // Insert, insert here
    /// The undo key.
    pub const Undo: KeySym = KeySym(0xff65);
    /// The redo key.
    pub const Redo: KeySym = KeySym(0xff66); // Redo, again
    /// The menu key.
    pub const Menu: KeySym = KeySym(0xff67);
    /// The find key.
    pub const Find: KeySym = KeySym(0xff68); // Find, search
    /// The cancel key.
    pub const Cancel: KeySym = KeySym(0xff69); // Cancel, stop, abort, exit
    /// The help key.
    pub const Help: KeySym = KeySym(0xff6a); // Help
    /// The break key.
    pub const Break: KeySym = KeySym(0xff6b);
    /// The switch key.
    pub const ModeSwitch: KeySym = KeySym(0xff7e); // Character set switch
    /// The num lock key.
    pub const Num_Lock: KeySym = KeySym(0xff7f);
    // Keypad functions, keypad numbers cleverly chosen to map to ASCII
    /// The space key on the numeric keypad.
    pub const NumPadSpace: KeySym = KeySym(0xff80); // Space
    /// The tab key on the numeric keypad.
    pub const NumPadTab: KeySym = KeySym(0xff89);
    /// The enter key on the numeric keypad.
    pub const NumPadEnter: KeySym = KeySym(0xff8d); // Enter
    /// The F1 key on the numeric keypad.
    pub const NumPadF1: KeySym = KeySym(0xff91); // PF1, NumPadA, ...
    /// The F2 key on the numeric keypad.
    pub const NumPadF2: KeySym = KeySym(0xff92);
    /// The F3 key on the numeric keypad.
    pub const NumPadF3: KeySym = KeySym(0xff93);
    /// The F4 key on the numeric keypad.
    pub const NumPadF4: KeySym = KeySym(0xff94);
    /// The home key on the numeric keypad.
    pub const NumPadHome: KeySym = KeySym(0xff95);
    /// The left key on the numeric keypad.
    pub const NumPadLeft: KeySym = KeySym(0xff96);
    /// The up key on the numeric keypad.
    pub const NumPadUp: KeySym = KeySym(0xff97);
    /// The right key on the numeric keypad.
    pub const NumPadRight: KeySym = KeySym(0xff98);
    /// The down key on the numeric keypad.
    pub const NumPadDown: KeySym = KeySym(0xff99);
    /// The prior key on the numeric keypad.
    pub const NumPadPrior: KeySym = KeySym(0xff9a);
    /// The up key on the numeric keypad.
    pub const NumPadPageUp: KeySym = KeySym(0xff9a);
    /// The next key on the numeric keypad.
    pub const NumPadNext: KeySym = KeySym(0xff9b);
    /// The page down key on the numeric keypad.
    pub const NumPadPageDown: KeySym = KeySym(0xff9b);
    /// The end key on the numeric keypad.
    pub const NumPadEnd: KeySym = KeySym(0xff9c);
    /// The begin key on the numeric keypad.
    pub const NumPadBegin: KeySym = KeySym(0xff9d);
    /// The insert key on the numeric keypad.
    pub const NumPadInsert: KeySym = KeySym(0xff9e);
    /// The delete key on the numeric keypad.
    pub const NumPadDelete: KeySym = KeySym(0xff9f);
    /// The equals key on the numeric keypad.
    pub const NumPadEqual: KeySym = KeySym(0xffbd); // Equals
    /// The multiply key on the numeric keypad.
    pub const NumPadMultiply: KeySym = KeySym(0xffaa);
    /// The addition key on the numeric keypad.
    pub const NumPadAdd: KeySym = KeySym(0xffab);
    /// The separator key on the numeric keypad.
    pub const NumPadSeparator: KeySym = KeySym(0xffac); // Separator, often comma
    /// The substract key on the numeric keypad.
    pub const NumPadSubtract: KeySym = KeySym(0xffad);
    /// The decimal divider key on the numeric keypad.
    pub const NumPadDecimal: KeySym = KeySym(0xffae);
    /// The divide key on the numeric keypad.
    pub const NumPadDivide: KeySym = KeySym(0xffaf);
    /// The 0 key on the numeric keypad.
    pub const NumPad0: KeySym = KeySym(0xffb0);
    /// The 1 key on the numeric keypad.
    pub const NumPad1: KeySym = KeySym(0xffb1);
    /// The 2 key on the numeric keypad.
    pub const NumPad2: KeySym = KeySym(0xffb2);
    /// The 3 key on the numeric keypad.
    pub const NumPad3: KeySym = KeySym(0xffb3);
    /// The 4 key on the numeric keypad.
    pub const NumPad4: KeySym = KeySym(0xffb4);
    /// The 5 key on the numeric keypad.
    pub const NumPad5: KeySym = KeySym(0xffb5);
    /// The 6 key on the numeric keypad.
    pub const NumPad6: KeySym = KeySym(0xffb6);
    /// The 7 key on the numeric keypad.
    pub const NumPad7: KeySym = KeySym(0xffb7);
    /// The 8 key on the numeric keypad.
    pub const NumPad8: KeySym = KeySym(0xffb8);
    /// The 9 key on the numeric keypad.
    pub const NumPad9: KeySym = KeySym(0xffb9);
    // Auxiliary functions; note the duplicate definitions for left and right
    // function keys;  Sun keyboards and a few other manufacturers have such
    // function key groups on the left and/or right sides of the keyboard.
    // We've not found a keyboard with more than 35 function keys total.
    /// The F1 key.
    pub const F1: KeySym = KeySym(0xffbe);
    /// The F2 key.
    pub const F2: KeySym = KeySym(0xffbf);
    /// The F3 key.
    pub const F3: KeySym = KeySym(0xffc0);
    /// The F4 key.
    pub const F4: KeySym = KeySym(0xffc1);
    /// The F5 key.
    pub const F5: KeySym = KeySym(0xffc2);
    /// The F6 key.
    pub const F6: KeySym = KeySym(0xffc3);
    /// The F7 key.
    pub const F7: KeySym = KeySym(0xffc4);
    /// The F8 key.
    pub const F8: KeySym = KeySym(0xffc5);
    /// The F9 key.
    pub const F9: KeySym = KeySym(0xffc6);
    /// The F10 key.
    pub const F10: KeySym = KeySym(0xffc7);
    /// The F11 key.
    pub const F11: KeySym = KeySym(0xffc8);
    /// The F12 key.
    pub const F12: KeySym = KeySym(0xffc9);
    /// The F13 key.
    pub const F13: KeySym = KeySym(0xffca);
    /// The F14 key.
    pub const F14: KeySym = KeySym(0xffcb);
    /// The F15 key.
    pub const F15: KeySym = KeySym(0xffcc);
    /// The F16 key.
    pub const F16: KeySym = KeySym(0xffcd);
    /// The F17 key.
    pub const F17: KeySym = KeySym(0xffce);
    /// The F18 key.
    pub const F18: KeySym = KeySym(0xffcf);
    /// The F19 key.
    pub const F19: KeySym = KeySym(0xffd0);
    /// The F20 key.
    pub const F20: KeySym = KeySym(0xffd1);
    /// The F21 key.
    pub const F21: KeySym = KeySym(0xffd2);
    /// The F22 key.
    pub const F22: KeySym = KeySym(0xffd3);
    /// The F23 key.
    pub const F23: KeySym = KeySym(0xffd4);
    /// The F24 key.
    pub const F24: KeySym = KeySym(0xffd5);
    // Modifiers
    /// The left shift key.
    pub const ShiftLeft: KeySym = KeySym(0xffe1); // Left shift
    /// The right shift key
    pub const ShiftRight: KeySym = KeySym(0xffe2); // Right shift
    /// The left control key.
    pub const ControlLeft: KeySym = KeySym(0xffe3); // Left control
    /// The right control key.
    pub const ControlRight: KeySym = KeySym(0xffe4); // Right control
    /// The caps lock key.
    pub const CapsLock: KeySym = KeySym(0xffe5); // Caps lock
    /// The shift lock key.
    pub const ShiftLock: KeySym = KeySym(0xffe6); // Shift lock
    /// The left meta key.
    pub const MetaLeft: KeySym = KeySym(0xffe7); // Left meta
    /// The right meta key.
    pub const MetaRight: KeySym = KeySym(0xffe8); // Right meta
    /// The left alt key.
    pub const AltLeft: KeySym = KeySym(0xffe9); // Left alt
    /// The right alt key.
    pub const AltRight: KeySym = KeySym(0xffea); // Right alt
    /// The left super key.
    pub const SuperLeft: KeySym = KeySym(0xffeb); // Left super
    /// The right super key.
    pub const SuperRight: KeySym = KeySym(0xffec); // Right super
    /// The left hyper key.
    pub const HyperLeft: KeySym = KeySym(0xffed); // Left hyper
    /// The right hyper key.
    pub const HyperRight: KeySym = KeySym(0xffee); // Right hyper
                                                   // Latin 1
                                                   // (ISO/IEC 8859-1 = Unicode U+0020..U+00FF)
                                                   // Byte 3 = 0
    /// The space key.
    pub const Space: KeySym = KeySym(0x0020); // U+0020 SPACE
    /// The exclamation mark (!) key.
    pub const Exclamation: KeySym = KeySym(0x0021); // U+0021 EXCLAMATION MARK
    /// The double quotation (") key.
    pub const Quote: KeySym = KeySym(0x0022); // U+0022 QUOTATION MARK
    /// The number sign (#) key.
    pub const NumberSign: KeySym = KeySym(0x0023); // U+0023 NUMBER SIGN
    /// The dollar sign ($) key.
    pub const Dollar: KeySym = KeySym(0x0024); // U+0024 DOLLAR SIGN
    /// The precent sign (%) key.
    pub const Percent: KeySym = KeySym(0x0025); // U+0025 PERCENT SIGN
    /// The ampersand (&amp;) key.
    pub const Ampersand: KeySym = KeySym(0x0026); // U+0026 AMPERSAND
    /// The apostrophe (') key.
    pub const Apostrophe: KeySym = KeySym(0x0027); // U+0027 APOSTROPHE
    /// The left parenthesis (() key.
    pub const ParenthesisLeft: KeySym = KeySym(0x0028); // U+0028 LEFT PARENTHESIS
    /// The right parenthesis ()) key.
    pub const ParenthesisRight: KeySym = KeySym(0x0029); // U+0029 RIGHT PARENTHESIS
    /// The aserisk (*) key.
    pub const Asterisk: KeySym = KeySym(0x002a); // U+002A ASTERISK
    /// The plus (+) key.
    pub const Plus: KeySym = KeySym(0x002b); // U+002B PLUS SIGN
    /// The comma (,) key.
    pub const Comma: KeySym = KeySym(0x002c); // U+002C COMMA
    /// The minus sign key.
    pub const Minus: KeySym = KeySym(0x002d); // U+002D HYPHEN-MINUS
    /// The full stop (.) key.
    pub const Period: KeySym = KeySym(0x002e); // U+002E FULL STOP
    /// The slash (/) key.
    pub const Slash: KeySym = KeySym(0x002f); // U+002F SOLIDUS
    /// The 0 key.
    pub const D0: KeySym = KeySym(0x0030); // U+0030 DIGIT ZERO
    /// The 1 key.
    pub const D1: KeySym = KeySym(0x0031); // U+0031 DIGIT ONE
    /// The 2 key.
    pub const D2: KeySym = KeySym(0x0032); // U+0032 DIGIT TWO
    /// The 3 key.
    pub const D3: KeySym = KeySym(0x0033); // U+0033 DIGIT THREE
    /// The 4 key.
    pub const D4: KeySym = KeySym(0x0034); // U+0034 DIGIT FOUR
    /// The 5 key.
    pub const D5: KeySym = KeySym(0x0035); // U+0035 DIGIT FIVE
    /// The 6 key.
    pub const D6: KeySym = KeySym(0x0036); // U+0036 DIGIT SIX
    /// The 7 key.
    pub const D7: KeySym = KeySym(0x0037); // U+0037 DIGIT SEVEN
    /// The 8 key.
    pub const D8: KeySym = KeySym(0x0038); // U+0038 DIGIT EIGHT
    /// The 9 key.
    pub const D9: KeySym = KeySym(0x0039); // U+0039 DIGIT NINE
    /// The colon (:) key.
    pub const Colon: KeySym = KeySym(0x003a); // U+003A COLON
    /// The semicolon (;) key.
    pub const Semicolon: KeySym = KeySym(0x003b); // U+003B SEMICOLON
    /// The less than (&lt;) key.
    pub const Less: KeySym = KeySym(0x003c); // U+003C LESS-THAN SIGN
    /// The equals (=) key.
    pub const Equal: KeySym = KeySym(0x003d); // U+003D EQUALS SIGN
    /// The greather than (&gt;) key.
    pub const Greater: KeySym = KeySym(0x003e); // U+003E GREATER-THAN SIGN
    /// The question mark (?) key.
    pub const Question: KeySym = KeySym(0x003f); // U+003F QUESTION MARK
    /// The at (@) key.
    pub const At: KeySym = KeySym(0x0040); // U+0040 COMMERCIAL AT
    /// The capital letter A key.
    pub const A: KeySym = KeySym(0x0041); // U+0041 LATIN CAPITAL LETTER A
    /// The capital letter B key.
    pub const B: KeySym = KeySym(0x0042); // U+0042 LATIN CAPITAL LETTER B
    /// The capital letter C key.
    pub const C: KeySym = KeySym(0x0043); // U+0043 LATIN CAPITAL LETTER C
    /// The capital letter D key.
    pub const D: KeySym = KeySym(0x0044); // U+0044 LATIN CAPITAL LETTER D
    /// The capital letter E key.
    pub const E: KeySym = KeySym(0x0045); // U+0045 LATIN CAPITAL LETTER E
    /// The capital letter F key.
    pub const F: KeySym = KeySym(0x0046); // U+0046 LATIN CAPITAL LETTER F
    /// The capital letter G key.
    pub const G: KeySym = KeySym(0x0047); // U+0047 LATIN CAPITAL LETTER G
    /// The capital letter H key.
    pub const H: KeySym = KeySym(0x0048); // U+0048 LATIN CAPITAL LETTER H
    /// The capital letter I key.
    pub const I: KeySym = KeySym(0x0049); // U+0049 LATIN CAPITAL LETTER I
    /// The capital letter J key.
    pub const J: KeySym = KeySym(0x004a); // U+004A LATIN CAPITAL LETTER J
    /// The capital letter K key.
    pub const K: KeySym = KeySym(0x004b); // U+004B LATIN CAPITAL LETTER K
    /// The capital letter L key.
    pub const L: KeySym = KeySym(0x004c); // U+004C LATIN CAPITAL LETTER L
    /// The capital letter M key.
    pub const M: KeySym = KeySym(0x004d); // U+004D LATIN CAPITAL LETTER M
    /// The capital letter N key.
    pub const N: KeySym = KeySym(0x004e); // U+004E LATIN CAPITAL LETTER N
    /// The capital letter O key.
    pub const O: KeySym = KeySym(0x004f); // U+004F LATIN CAPITAL LETTER O
    /// The capital letter P key.
    pub const P: KeySym = KeySym(0x0050); // U+0050 LATIN CAPITAL LETTER P
    /// The capital letter Q key.
    pub const Q: KeySym = KeySym(0x0051); // U+0051 LATIN CAPITAL LETTER Q
    /// The capital letter R key.
    pub const R: KeySym = KeySym(0x0052); // U+0052 LATIN CAPITAL LETTER R
    /// The capital letter S key.
    pub const S: KeySym = KeySym(0x0053); // U+0053 LATIN CAPITAL LETTER S
    /// The capital letter T key.
    pub const T: KeySym = KeySym(0x0054); // U+0054 LATIN CAPITAL LETTER T
    /// The capital letter U key.
    pub const U: KeySym = KeySym(0x0055); // U+0055 LATIN CAPITAL LETTER U
    /// The capital letter V key.
    pub const V: KeySym = KeySym(0x0056); // U+0056 LATIN CAPITAL LETTER V
    /// The capital letter W key.
    pub const W: KeySym = KeySym(0x0057); // U+0057 LATIN CAPITAL LETTER W
    /// The capital letter X key.
    pub const X: KeySym = KeySym(0x0058); // U+0058 LATIN CAPITAL LETTER X
    /// The capital letter Y key.
    pub const Y: KeySym = KeySym(0x0059); // U+0059 LATIN CAPITAL LETTER Y
    /// The capital letter Z key.
    pub const Z: KeySym = KeySym(0x005a); // U+005A LATIN CAPITAL LETTER Z
    /// The left square bracket ([) key.
    pub const BracketLeft: KeySym = KeySym(0x005b); // U+005B LEFT SQUARE BRACKET
    /// The backslash (\) key.
    pub const Backslash: KeySym = KeySym(0x005c); // U+005C REVERSE SOLIDUS
    /// The right square bracket (]) key.
    pub const Bracketright: KeySym = KeySym(0x005d); // U+005D RIGHT SQUARE BRACKET
    /// The cirumflex accent (^) key.
    pub const AsciiCircum: KeySym = KeySym(0x005e); // U+005E CIRCUMFLEX ACCENT
    /// The underscore (_) key.
    pub const Underscore: KeySym = KeySym(0x005f); // U+005F LOW LINE
    /// The grave accent (`) sign.
    pub const Grave: KeySym = KeySym(0x0060); // U+0060 GRAVE ACCENT
    /// The small letter a key.
    pub const a: KeySym = KeySym(0x0061); // U+0061 LATIN SMALL LETTER A
    /// The small letter b key.
    pub const b: KeySym = KeySym(0x0062); // U+0062 LATIN SMALL LETTER B
    /// The small letter c key.
    pub const c: KeySym = KeySym(0x0063); // U+0063 LATIN SMALL LETTER C
    /// The small letter d key.
    pub const d: KeySym = KeySym(0x0064); // U+0064 LATIN SMALL LETTER D
    /// The small letter e key.
    pub const e: KeySym = KeySym(0x0065); // U+0065 LATIN SMALL LETTER E
    /// The small letter f key.
    pub const f: KeySym = KeySym(0x0066); // U+0066 LATIN SMALL LETTER F
    /// The small letter g key.
    pub const g: KeySym = KeySym(0x0067); // U+0067 LATIN SMALL LETTER G
    /// The small letter h key.
    pub const h: KeySym = KeySym(0x0068); // U+0068 LATIN SMALL LETTER H
    /// The small letter i key.
    pub const i: KeySym = KeySym(0x0069); // U+0069 LATIN SMALL LETTER I
    /// The small letter j key.
    pub const j: KeySym = KeySym(0x006a); // U+006A LATIN SMALL LETTER J
    /// The small letter k key.
    pub const k: KeySym = KeySym(0x006b); // U+006B LATIN SMALL LETTER K
    /// The small letter l key.
    pub const l: KeySym = KeySym(0x006c); // U+006C LATIN SMALL LETTER L
    /// The small letter m key.
    pub const m: KeySym = KeySym(0x006d); // U+006D LATIN SMALL LETTER M
    /// The small letter n key.
    pub const n: KeySym = KeySym(0x006e); // U+006E LATIN SMALL LETTER N
    /// The small letter o key.
    pub const o: KeySym = KeySym(0x006f); // U+006F LATIN SMALL LETTER O
    /// The small letter p key.
    pub const p: KeySym = KeySym(0x0070); // U+0070 LATIN SMALL LETTER P
    /// The small letter q key.
    pub const q: KeySym = KeySym(0x0071); // U+0071 LATIN SMALL LETTER Q
    /// The small letter r key.
    pub const r: KeySym = KeySym(0x0072); // U+0072 LATIN SMALL LETTER R
    /// The small letter s key.
    pub const s: KeySym = KeySym(0x0073); // U+0073 LATIN SMALL LETTER S
    /// The small letter t key.
    pub const t: KeySym = KeySym(0x0074); // U+0074 LATIN SMALL LETTER T
    /// The small letter u key.
    pub const u: KeySym = KeySym(0x0075); // U+0075 LATIN SMALL LETTER U
    /// The small letter v key.
    pub const v: KeySym = KeySym(0x0076); // U+0076 LATIN SMALL LETTER V
    /// The small letter w key.
    pub const w: KeySym = KeySym(0x0077); // U+0077 LATIN SMALL LETTER W
    /// The small letter x key.
    pub const x: KeySym = KeySym(0x0078); // U+0078 LATIN SMALL LETTER X
    /// The small letter y key.
    pub const y: KeySym = KeySym(0x0079); // U+0079 LATIN SMALL LETTER Y
    /// The small letter z key.
    pub const z: KeySym = KeySym(0x007a); // U+007A LATIN SMALL LETTER Z
    /// The left curly bracket ({) key.
    pub const BraceLeft: KeySym = KeySym(0x007b); // U+007B LEFT CURLY BRACKET
    /// The vertical bar (|) key.
    pub const Bar: KeySym = KeySym(0x007c); // U+007C VERTICAL LINE
    /// The right curly bracket (}) key.
    pub const BraceRight: KeySym = KeySym(0x007d); // U+007D RIGHT CURLY BRACKET
    /// The tilde (~) key.
    pub const AsciiTilde: KeySym = KeySym(0x007e); // U+007E TILDE
}
