import assert from "node:assert/strict";

export async function flattenRegressions(page, reference, check, helpers, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, captureNativeScreenshot } = helpers;
    const fill = (r, g, b) => ({ paint: { type: "solid", r, g, b, a: 255 }, opacity: 1, visible: true });
    const action = async () => {
        await clickNative(page, "ActionsButton");
        await clickNative(page, "ActionSearch", "ModalViewport");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText("Flatten to vector");
        await clickNative(page, "Action-flatten", "ModalViewport");
        await frames(page);
    };
    const firstRoot = async (id) => {
        await page.keyboard.press("Escape");
        await page.keyboard.press("Tab");
        await page.waitForFunction((id) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).selection.join() === String(id), id);
    };
    const roundTrip = async (before, after, depth) => {
        assert.equal((await state(page)).historyLength, depth + 1, "Flatten records exactly one transaction");
        await page.keyboard.press("Control+z"); await frames(page);
        assert.deepEqual(await documentState(page), before, "Flatten undo restores exact original document");
        await page.keyboard.press("Control+Shift+z"); await frames(page);
        assert.deepEqual(await documentState(page), after, "Flatten redo restores every outlined field");
    };

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "text", name: "Shaped outline", x: 40, y: 40,
            width: 240, height: 130, text: "office\nFerroUI", font_family: "Inter", font_size: 32,
            text_align: "center", text_decoration: "underline", letter_spacing: 1.5,
            text_runs: [], fills: [fill(210, 50, 100)], rotation: 8, corner_radius: 0 }),
    ], "Native flatten text"));
    await firstRoot(2);
    const textBefore = await documentState(page), textDepth = (await state(page)).historyLength;
    await action();
    const textAfter = await documentState(page), outlined = textAfter.nodes[2];
    assert.equal(outlined.kind, "vector");
    assert.equal(outlined.parent, textBefore.nodes[2].parent);
    assert.deepEqual(textAfter.pages[0].children, [2]);
    assert.deepEqual(outlined.fills, textBefore.nodes[2].fills);
    assert.equal(outlined.rotation, 8);
    assert.ok(outlined.path.segments.length > 20, "Shaped glyphs and decorations become editable curve commands");
    await captureNativeScreenshot(mode + "-flatten-text", { controls: ["CanvasHost"] });
    await roundTrip(textBefore, textAfter, textDepth);
    check("Native Flatten to vector converts real shaped, rotated multiline text and decorations through public FerroUI outlines while retaining identity, paint, parent and one-step undo/redo");

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "boolean", name: "Outlined difference", x: 40, y: 40,
            width: 200, height: 140, children: [3, 4], boolean_operation: "subtract", fills: [], corner_radius: 0 }),
        fixtureNode(reference, 3, { parent: 2, name: "Outer shape", x: 0, y: 0,
            width: 200, height: 140, fills: [fill(30, 170, 100)], corner_radius: 0 }),
        fixtureNode(reference, 4, { parent: 2, kind: "ellipse", name: "Hole", x: 60, y: 30,
            width: 80, height: 80, fills: [fill(230, 90, 30)], corner_radius: 0 }),
    ], "Native flatten boolean"));
    await firstRoot(2);
    const booleanBefore = await documentState(page), booleanDepth = (await state(page)).historyLength;
    await action();
    const booleanAfter = await documentState(page);
    assert.equal(booleanAfter.nodes[2].kind, "vector");
    assert.deepEqual(Object.keys(booleanAfter.nodes), ["2"]);
    assert.deepEqual(booleanAfter.nodes[2].fills, booleanBefore.nodes[3].fills);
    assert.ok(booleanAfter.nodes[2].path.segments.length >= 8);
    await roundTrip(booleanBefore, booleanAfter, booleanDepth);
    check("Native Flatten evaluates Boolean difference into editable curves, retains the operand-inherited fill and surviving ID, removes operands atomically, and exactly restores them on Undo");

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { name: "Red region", x: 40, y: 40, width: 80, height: 80,
            fills: [fill(220, 40, 60)], corner_radius: 0 }),
        fixtureNode(reference, 3, { name: "Blue region", x: 160, y: 40, width: 80, height: 80,
            fills: [fill(20, 120, 230)], corner_radius: 0 }),
    ], "Atomic flatten rejection"));
    await clickWorld(page, 70, 70);
    await page.keyboard.down("Shift");
    try { await clickWorld(page, 190, 70); } finally { await page.keyboard.up("Shift"); }
    const invalidBefore = await documentState(page), invalidDepth = (await state(page)).historyLength;
    assert.equal((await state(page)).selectionCount, 2);
    await action();
    assert.deepEqual(await documentState(page), invalidBefore, "A heterogeneous paint batch must never discard one region's paint");
    assert.equal((await state(page)).historyLength, invalidDepth);
    check("Native Flatten rejects independently colored regions without changing the document or history instead of silently discarding their appearance");
}
