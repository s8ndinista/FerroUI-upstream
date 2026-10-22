import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Native TextBox selection, ComboBoxes, one Apply click per edit, actual
// presenter geometry and downloaded native PNG. No command mutates paragraph
// state or injects a caret/selection into the application.
export async function paragraphRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, waitForNativeFrame, waitForNativeScene,
        captureNativeScreenshot } = helpers;
    const waitDocument = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const doc = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("doc", "args", "return (" + source + ")(doc, args)")(doc, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const click = name => clickNative(page, name, "ModalViewport");
    const choose = (name, index) => chooseNative(page, name, { index, scrollContainer: "ModalViewport" });
    const field = async (name, value) => {
        await click(name);
        assert.equal((await state(page)).chrome[name].focused, true, name + " owns native focus");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(String(value));
    };
    const numeric = async (name, value) => {
        await choose(name + "Mode", 1);
        await field(name, value);
    };
    const typography = async name => {
        await page.waitForFunction(name => {
            const control = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome[name];
            return control?.visible && control.measureValid && control.arrangeValid && control.typography?.lineCount > 0;
        }, name, { timeout: 10000 });
        return (await state(page)).chrome[name].typography;
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        let view = await state(page);
        await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " undo");
        assert.deepEqual(await documentState(page), before);
        view = await state(page);
        await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " redo");
        assert.deepEqual(await documentState(page), after);
    };

    const value = "A🦀 first\nשלום second\nLast";
    const firstEnd = Buffer.byteLength("A🦀 first\n");
    const secondEnd = Buffer.byteLength("A🦀 first\nשלום second\n");
    const text = fixtureNode(reference, 2, {
        kind: "text", name: "Independent paragraphs", x: 40, y: 40, width: 440, height: 200,
        text: value, text_runs: [], text_paragraphs: [], font_family: "Inter", font_size: 20,
        font_weight: 400, italic: false, letter_spacing: 0, line_height: 1.2,
        text_direction: "auto", text_auto_resize: "fixed", text_vertical_align: "top",
        text_decoration: "none", text_align: "left", font_features: [],
        fills: [{ paint: { type: "solid", r: 20, g: 20, b: 20, a: 255 }, opacity: 1, visible: true }],
        strokes: [], effects: [],
    });
    await importFixture(page, isolatedDocument(reference, [text], "Native independent paragraphs"));
    await clickWorld(page, 65, 60);
    assert.deepEqual((await state(page)).selection, [2]);
    let beforeOpen = await state(page);
    await clickNative(page, "RichTextButton", "InspectorViewport");
    await typography("RichTextSelection");
    await waitForNativeScene(page, beforeOpen.sceneFrames, "Rich text dialog publication");
    beforeOpen = await state(page);
    await click("RichTextParagraphs");
    await typography("ParagraphSelection");
    await waitForNativeScene(page, beforeOpen.sceneFrames, "Paragraph dialog publication");
    const before = await documentState(page), depth = (await state(page)).historyLength;
    await click("ParagraphSelection");
    await page.keyboard.press("Control+Home");
    await page.keyboard.press("Shift+End");
    let layout = await typography("ParagraphSelection");
    assert.deepEqual([Math.min(layout.selectionStart, layout.selectionEnd), Math.max(layout.selectionStart, layout.selectionEnd)], [0, 9]);
    await choose("ParagraphAlignment", 2);
    await choose("ParagraphDirection", 3);
    await numeric("ParagraphLineHeight", 1.5);
    await numeric("ParagraphLetterSpacing", 0.5);
    await numeric("ParagraphBefore", 5);
    await numeric("ParagraphAfter", 20);
    await numeric("ParagraphIndent", 14);
    await clickNative(page, "ParagraphApply", "ModalActions");
    await waitDocument(doc => doc.nodes[2].text_paragraphs?.[0]?.indent === 14);
    const first = { start: 0, end: firstEnd, alignment: "center", direction: "auto",
        line_height: 1.5, letter_spacing: 0.5, space_before: 5, space_after: 20, indent: 14 };
    let authored = await documentState(page);
    assert.deepEqual(authored.nodes[2].text_paragraphs, [first]);
    assert.equal(authored.nodes[2].text, value);
    assert.deepEqual(authored.nodes[2].text_runs, []);
    assert.equal((await state(page)).historyLength, depth + 1);
    layout = await typography("ParagraphSelection");
    assert.equal(layout.lineCount, 3);
    assert.deepEqual(layout.lines.map(line => [line.x, line.y, line.height, line.direction]), [
        [14, 5, 30, "LeftToRight"], [0, 55, 24, "RightToLeft"], [0, 79, 24, "LeftToRight"],
    ]);
    assert.equal(layout.layoutHeight, 103);
    check("Native UTF-16 selection across a non-BMP character authors one complete UTF-8 paragraph with independent alignment, Auto direction, line height, letter spacing, before/after spacing and first-line indentation in the actual TextPresenter");

    await click("ParagraphSelection");
    await page.keyboard.press("Control+Home");
    await page.keyboard.press("ArrowDown");
    layout = await typography("ParagraphSelection");
    assert.equal(layout.selectionStart, layout.selectionEnd);
    assert.ok(layout.caretIndex >= 10 && layout.caretIndex < 22, "Native vertical navigation reaches the second paragraph across its spacing");
    assert.equal(layout.caret.y, 55, "The vertical caret uses the actual second-paragraph origin");
    // Make the authored target independent of the physical X that vertical
    // navigation preserved across mixed-direction text.
    await page.keyboard.press("Control+Home");
    await page.keyboard.press("End");
    await page.keyboard.press("ArrowRight");
    assert.equal((await typography("ParagraphSelection")).caretIndex, 10);
    await choose("ParagraphAlignment", 3);
    await choose("ParagraphDirection", 2);
    await numeric("ParagraphLineHeight", 2);
    await choose("ParagraphLetterSpacingMode", 0);
    await numeric("ParagraphBefore", 7);
    await numeric("ParagraphAfter", 11);
    await numeric("ParagraphIndent", 10);
    await clickNative(page, "ParagraphApply", "ModalActions");
    await waitDocument(doc => doc.nodes[2].text_paragraphs?.length === 2);
    authored = await documentState(page);
    assert.deepEqual(authored.nodes[2].text_paragraphs, [first, {
        start: firstEnd, end: secondEnd, alignment: "right", direction: "rtl", line_height: 2,
        space_before: 7, space_after: 11, indent: 10,
    }]);
    assert.equal((await state(page)).historyLength, depth + 2);
    layout = await typography("ParagraphSelection");
    assert.deepEqual(layout.lines.map(line => [line.x, line.y, line.height]), [[14,5,30],[0,62,40],[0,113,24]]);
    await field("ParagraphIndent", -1);
    await clickNative(page, "ParagraphApply", "ModalActions");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ParagraphError?.visible,
        undefined, { timeout: 10000 });
    assert.deepEqual(await documentState(page), authored, "Invalid paragraph metrics reject the entire patch");
    assert.equal((await state(page)).historyLength, depth + 2);
    await field("ParagraphIndent", 10);
    await clickNative(page, "ParagraphApply", "ModalActions");
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ParagraphError?.visible,
        undefined, { timeout: 10000 });
    assert.equal((await state(page)).historyLength, depth + 2, "An unchanged native Apply adds no history");
    await captureNativeScreenshot(mode + "-native-paragraph-authoring", { controls: ["ParagraphSelection", "ParagraphApply"] });
    check("A collapsed native caret styles only its paragraph; RTL alignment and spacing move the real layout, invalid metrics fail atomically and repeated Apply remains a no-op");

    await clickNative(page, "ParagraphInheritAll", "ModalActions");
    await waitDocument(doc => doc.nodes[2].text_paragraphs?.length === 1);
    const inherited = await documentState(page);
    assert.deepEqual(inherited.nodes[2].text_paragraphs, [first]);
    await clickNative(page, "ParagraphDone", "ModalActions");
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ModalBackdrop.visible,
        undefined, { timeout: 10000 });
    await history(authored, inherited, depth + 2, "Inherit second paragraph");
    const [download] = await Promise.all([
        page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport"),
    ]);
    assert.equal(await download.failure(), null);
    const pngPath = resolve(output, mode + "-paragraph-artwork.png");
    await download.saveAs(pngPath);
    const png = PNG.sync.read(await readFile(pngPath));
    assert.deepEqual([png.width, png.height], [440, 200]);
    const ink = (from, to) => {
        let count = 0;
        for (let y = from; y < to; y++) for (let x = 0; x < png.width; x++) count += Number(png.data[(y * png.width + x) * 4 + 3] > 16);
        return count;
    };
    assert.ok(ink(5, 35) > 40 && ink(55, 79) > 40 && ink(79, 103) > 20, "Actual native PNG contains all three independently positioned paragraphs");
    assert.equal(ink(40, 50), 0, "Authored paragraph spacing leaves a real transparent gap in exported artwork");
    assert.deepEqual(await documentState(page), inherited, "Native export preserves the exact authored paragraph document");
    check("Paragraph Inherit has exact one-step undo/redo and preserves the first paragraph; actual downloaded native PNG contains the three text bands and the authored transparent spacing gap");

    const enterInline = async () => {
        await clickWorld(page, 65, 60);
        assert.equal((await state(page)).chrome.CanvasHost.focusWithin, true);
        const before = await state(page);
        await page.keyboard.press("Enter");
        await page.waitForFunction(() => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
            return view.editingText === 2 && view.chrome.InlineTextBox?.focused && view.chrome.InlineTextBox?.visible;
        }, undefined, { timeout: 10000 });
        await waitForNativeFrame(page, before.renderedFrames, "Native paragraph inline entry");
        await waitForNativeScene(page, before.sceneFrames, "Native paragraph inline entry");
    };
    const splitFirst = async () => {
        await page.keyboard.press("Control+Home");
        await page.keyboard.press("ArrowRight");
        await page.keyboard.press("ArrowRight");
        assert.equal((await typography("InlineTextBox")).caretIndex, 3);
        await page.keyboard.press("Enter");
        await waitDocument(doc => doc.nodes[2].text === "A🦀\n first\nשלום second\nLast");
        const split = await documentState(page);
        assert.deepEqual(split.nodes[2].text_paragraphs, [{ ...first, end: 6 }, { ...first, start: 6, end: firstEnd + 1 }]);
        const native = await typography("InlineTextBox");
        assert.equal(native.lineCount, 4);
        assert.deepEqual(native.lines.slice(0,2).map(line => [line.x, line.y, line.height]), [[14,5,30],[14,60,30]]);
        return split;
    };
    await enterInline();
    await splitFirst();
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    assert.deepEqual(await documentState(page), inherited);
    const splitDepth = (await state(page)).historyLength;
    await enterInline();
    const split = await splitFirst();
    await captureNativeScreenshot(mode + "-native-paragraph-inline", { controls: ["InlineTextBox"] });
    await page.keyboard.press("Control+Enter");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    await history(inherited, split, splitDepth, "Native paragraph split");
    await importFixture(page, split);
    assert.deepEqual(await documentState(page), split);
    assert.equal(before.nodes[2].text, split.nodes[2].text.replace("🦀\n", "🦀"));
    check("Real native Unicode editing splits paragraph styles with correct UTF-8 remapping and shared line/caret offsets; cancel restores the original, commit is one exact undo/redo step and JSON round-trip preserves every paragraph field");
}
