import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Author through the native inspector, export through the public FerroUI path.
export async function cornerRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, captureNativeScreenshot } = helpers;
    const edit = async value => {
        await clickNative(page, "CornerSmoothingField", "InspectorViewport");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await page.keyboard.press("Enter");
        await frames(page);
    };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        name: "Continuous corners", x: 40, y: 40, width: 200, height: 140,
        corner_radius: 40, corner_radii: null, corner_smoothing: 0, effects: [], strokes: [],
        fills: [{ paint: { type: "solid", r: 77, g: 63, b: 230, a: 255 }, opacity: 1, visible: true }],
    })], "Native corner smoothing"));
    await clickWorld(page, 140, 100);
    const before = await documentState(page), depth = (await state(page)).historyLength;
    await edit("60");
    const after = await documentState(page);
    assert.equal(after.nodes[2].corner_smoothing, 0.6);
    assert.equal((await state(page)).historyLength, depth + 1);
    await edit("101");
    assert.deepEqual(await documentState(page), after);
    assert.equal((await state(page)).historyLength, depth + 1);
    assert.equal((await state(page)).chrome.CornerSmoothingField.text, "101");
    await page.keyboard.press("Escape");
    await frames(page);
    assert.equal((await state(page)).chrome.CornerSmoothingField.text, "60");
    await captureNativeScreenshot(mode + "-corner-smoothing", { controls: ["CornerSmoothingField"] });
    await clickWorld(page, 140, 100);
    await page.keyboard.press("Control+z"); await frames(page);
    assert.deepEqual(await documentState(page), before);
    await page.keyboard.press("Control+Shift+z"); await frames(page);
    assert.deepEqual(await documentState(page), after);
    check("Native smoothing commits one history step, rejects out-of-range drafts atomically, and restores exact document fields through undo/redo");

    const download = async (button, suffix) => {
        const [item] = await Promise.all([page.waitForEvent("download", { timeout: 60000 }),
            clickNative(page, button, "InspectorViewport")]);
        assert.equal(await item.failure(), null);
        const path = resolve(output, mode + "-smoothed-corners." + suffix);
        await item.saveAs(path);
        return readFile(path);
    };
    const png = PNG.sync.read(await download("ExportPngButton", "png"));
    const svg = (await download("ExportSvgButton", "svg")).toString("utf8");
    assert.deepEqual([png.width, png.height], [200, 140]);
    assert.equal(png.data[3], 0);
    assert.equal(png.data[(70 * png.width + 100) * 4 + 3], 255);
    const external = await page.evaluate(async source => {
        const parsed = new DOMParser().parseFromString(source, "image/svg+xml");
        const paths = [...parsed.querySelectorAll("path")].map(p => p.getAttribute("d"));
        const url = URL.createObjectURL(new Blob([source], { type: "image/svg+xml" }));
        try {
            const image = new Image(); image.src = url; await image.decode();
            const canvas = document.createElement("canvas");
            canvas.width = image.width; canvas.height = image.height;
            const context = canvas.getContext("2d"); context.drawImage(image, 0, 0);
            const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
            return { width: canvas.width, height: canvas.height, paths,
                alpha: Array.from({ length: pixels.length / 4 }, (_, i) => pixels[i * 4 + 3]) };
        } finally { URL.revokeObjectURL(url); }
    }, svg);
    assert.deepEqual([external.width, external.height], [200, 140]);
    assert.ok(external.paths.some(path => (path.match(/C/g) || []).length === 12),
        "SVG retains the four cubic/arc/cubic corners");
    let edgeDifferences = 0;
    for (let i = 0; i < external.alpha.length; i++) {
        if (Math.abs(external.alpha[i] - png.data[i * 4 + 3]) > 12) edgeDifferences++;
    }
    assert.ok(edgeDifferences <= 48, "Independent SVG decoder and FerroUI PNG share the smooth silhouette: " + edgeDifferences);
    assert.deepEqual(await documentState(page), after, "Both exports preserve editable corner parameters");
    check("Public FerroUI PNG and independently decoded SVG retain the same twelve-cubic smoothed silhouette and transparent exterior");
}
