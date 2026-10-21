import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Author square-grid guides and their shared ownership through FerroUI controls.
// Screenshots verify the editor overlay; the real PNG export must exclude it.
export async function gridStyleRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, waitForNativeFrame, captureNativeScreenshot } = helpers;
    const white = { paint: { type: "solid", r: 255, g: 255, b: 255, a: 255 }, opacity: 1, visible: true };
    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Local visibility frame", x: 40, y: 40,
            width: 160, height: 120, corner_radius: 0, corner_radii: null, fills: [white], layout_grid: null, grid_style: null }),
        fixtureNode(reference, 3, { kind: "frame", name: "Linked grid frame", x: 260, y: 40,
            width: 160, height: 120, corner_radius: 0, corner_radii: null, fills: [white], layout_grid: null, grid_style: null }),
    ], "Native shared layout grids"));
    const select = async (ids) => {
        await clickNative(page, "SelectTool");
        const frame = (await state(page)).renderedFrames;
        await page.keyboard.press("Escape");
        await waitForNativeFrame(page, frame, "Clear grid selection");
        for (const [index, id] of ids.entries()) {
            if (index) await page.keyboard.down("Shift");
            try { await clickWorld(page, id === 2 ? 80 : 300, 80); }
            finally { if (index) await page.keyboard.up("Shift"); }
        }
        assert.deepEqual((await state(page)).selection, ids);
    };
    const field = async (name, value) => {
        await clickNative(page, name, "ModalViewport");
        assert.equal((await state(page)).chrome[name].focused, true);
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await page.keyboard.press("Enter");
        await frames(page);
    };
    const openGrid = () => clickNative(page, "LayoutGridSettingsButton", "InspectorViewport");
    const closeGrid = () => clickNative(page, "LayoutGridDone", "ModalActions");
    const openStyle = async id => {
        await clickNative(page, "ActionsButton");
        await clickNative(page, "Action-styles", "ModalViewport");
        await clickNative(page, "StyleEdit" + id, "ModalViewport");
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        let frame = (await state(page)).renderedFrames;
        await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, frame, label + " undo");
        assert.deepEqual(await documentState(page), before);
        frame = (await state(page)).renderedFrames;
        await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, frame, label + " redo");
        assert.deepEqual(await documentState(page), after);
    };

    await select([2, 3]);
    await openGrid();
    const beforeAdd = await documentState(page), addDepth = (await state(page)).historyLength;
    await clickNative(page, "LayoutGridAdd", "ModalViewport");
    await page.waitForFunction(() => {
        const doc = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return doc.nodes[2].layout_grid && doc.nodes[3].layout_grid;
    });
    const added = await documentState(page);
    assert.equal(added.nodes[2].layout_grid.size, 8);
    assert.deepEqual(added.nodes[2].layout_grid, added.nodes[3].layout_grid);
    await closeGrid();
    await history(beforeAdd, added, addDepth, "Add frame grids");
    await openGrid();
    await field("LayoutGridSize", "12.5");
    await field("LayoutGridColor", "EF444480");
    const beforeCreate = await documentState(page), createDepth = (await state(page)).historyLength;
    await clickNative(page, "LayoutGridCreateStyle", "ModalActions");
    const created = await documentState(page), id = created.nodes[2].grid_style;
    assert.ok(id > 0);
    assert.equal(created.nodes[3].grid_style, id);
    assert.equal(created.styles[id].value.type, "grid");
    assert.equal(created.styles[id].value.size, 12.5);
    assert.deepEqual(created.styles[id].value.color, { r: 239, g: 68, b: 68, a: 128 });
    await clickNative(page, "StyleDone", "ModalActions");
    await history(beforeCreate, created, createDepth, "Create shared grid style");
    check("Native frame-grid settings add and edit both selected frames, create a shared grid style with RGBA color and fractional spacing, and preserve exact single-step history");

    await openStyle(id);
    const beforeSize = await documentState(page), sizeDepth = (await state(page)).historyLength;
    await field("StyleGridSize", "24");
    const resized = await documentState(page);
    for (const node of [2, 3]) {
        assert.equal(resized.nodes[node].grid_style, id);
        assert.equal(resized.nodes[node].layout_grid.size, 24);
    }
    await captureNativeScreenshot(mode + "-native-grid-style", { controls: ["ModalPanel", "StyleGridSize", "StyleGridColor"] });
    await clickNative(page, "StyleDone", "ModalActions");
    await history(beforeSize, resized, sizeDepth, "Shared grid spacing");
    await select([2]);
    await openGrid();
    const beforeVisibility = await documentState(page), visibilityDepth = (await state(page)).historyLength;
    await clickNative(page, "LayoutGridVisible", "ModalViewport");
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].layout_grid.visible);
    const hidden = await documentState(page);
    assert.equal(hidden.nodes[2].grid_style, id);
    assert.equal(hidden.nodes[3].layout_grid.visible, true);
    await closeGrid();
    await history(beforeVisibility, hidden, visibilityDepth, "Local grid visibility");
    check("Shared grid-size edits propagate through both frame links; hiding one frame's overlay retains its shared style and the other frame's visibility with exact undo/redo");

    await openGrid();
    const invalidDepth = (await state(page)).historyLength;
    for (const invalid of ["0", "NaN", "-1"]) {
        await field("LayoutGridSize", invalid);
        assert.equal((await state(page)).chrome.LayoutGridSizeError.visible, true);
        assert.deepEqual(await documentState(page), hidden);
        assert.equal((await state(page)).historyLength, invalidDepth);
    }
    await closeGrid();
    assert.equal((await state(page)).chrome.ModalBackdrop.visible, true, "A rejected grid draft keeps its native editor open");
    await clickNative(page, "LayoutGridSize", "ModalViewport");
    await page.keyboard.press("Escape");
    await frames(page);
    assert.equal((await state(page)).chrome.LayoutGridSize.text, "24");
    assert.equal((await state(page)).chrome.LayoutGridSizeError.visible, false);
    await closeGrid();
    await openStyle(id);
    await field("StyleGridColor", "2563EBCC");
    const recolored = await documentState(page);
    assert.equal(recolored.nodes[2].layout_grid.visible, false);
    for (const node of [2, 3]) assert.deepEqual(recolored.nodes[node].layout_grid.color, { r: 37, g: 99, b: 235, a: 204 });
    await clickNative(page, "StyleDone", "ModalActions");
    await openGrid();
    const beforeDetach = await documentState(page), detachDepth = (await state(page)).historyLength;
    await field("LayoutGridSize", "18");
    const detached = await documentState(page);
    assert.equal(detached.nodes[2].grid_style ?? null, null);
    assert.equal(detached.nodes[2].layout_grid.size, 18);
    assert.equal(detached.nodes[3].grid_style, id);
    assert.equal(detached.nodes[3].layout_grid.size, 24);
    await closeGrid();
    await history(beforeDetach, detached, detachDepth, "Local grid-size override");
    await openStyle(id);
    await field("StyleGridSize", "32");
    const independent = await documentState(page);
    assert.deepEqual(independent.nodes[2].layout_grid, detached.nodes[2].layout_grid);
    assert.equal(independent.nodes[3].layout_grid.size, 32);
    await clickNative(page, "StyleDone", "ModalActions");
    check("Native grid fields reject nonpositive/nonfinite sizes atomically, preserve canceled drafts and style links, and detach only a frame whose spacing is explicitly overridden");

    await select([]);
    await page.mouse.move(4, 4);
    const view = await state(page);
    const pixels = PNG.sync.read(await captureNativeScreenshot(mode + "-layout-grid-overlay", { controls: ["CanvasHost"] }));
    const point = { x: view.chrome.CanvasHost.x + view.pan.x + 292 * view.zoom,
        y: view.chrome.CanvasHost.y + view.pan.y + 85 * view.zoom };
    let blueInk = 0;
    for (let y = Math.round(point.y) - 3; y <= Math.round(point.y) + 3; y++) {
        for (let x = Math.round(point.x) - 3; x <= Math.round(point.x) + 3; x++) {
            const offset = (y * pixels.width + x) * 4;
            blueInk += Number(pixels.data[offset + 2] > pixels.data[offset] + 35 &&
                pixels.data[offset + 2] > pixels.data[offset + 1] + 20);
        }
    }
    assert.ok(blueInk > 0, "The actual native canvas must draw the linked blue guide at its 32-unit spacing");
    await select([3]);
    const [download] = await Promise.all([
        page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport"),
    ]);
    assert.equal(await download.failure(), null);
    const path = resolve(output, mode + "-grid-free-artwork.png");
    await download.saveAs(path);
    const png = PNG.sync.read(await readFile(path));
    assert.deepEqual([png.width, png.height], [160, 120]);
    assert.ok(png.data.every(channel => channel === 255), "Grid guides must not enter the exported white artwork");
    assert.deepEqual(await documentState(page), independent, "Selection and exports leave the authored grids untouched");
    await importFixture(page, independent);
    assert.deepEqual(await documentState(page), independent);
    check("The browser surface renders the linked grid at its authored spacing while actual PNG artwork excludes every guide pixel; grid definitions, local visibility and bindings survive document round-trip");
}
