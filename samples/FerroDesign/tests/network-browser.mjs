import assert from "node:assert/strict";
import { PNG } from "pngjs";

// Pointer coordinates and controls are supplied by the actual FerroUI tree.
export async function vectorNetworkRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, clickWorld, clickNative, frames, captureNativeScreenshot } = helpers;
    const command = async (name) => {
        assert.equal(await page.evaluate((name) => globalThis.ferroDesign.ferroDesignCommand(name), name), true);
        await frames(page);
    };
    const node = async () => (await documentState(page)).nodes[2];
    const screen = (view, x, y) => ({ x: view.chrome.CanvasHost.x + view.pan.x + x * view.zoom, y: view.chrome.CanvasHost.y + view.pan.y + y * view.zoom });
    const drag = async (from, to) => {
        const view = await state(page), a = screen(view, ...from), b = screen(view, ...to);
        await page.mouse.move(a.x, a.y);
        await page.mouse.down();
        await page.mouse.move(b.x, b.y, { steps: 16 });
        await page.mouse.up();
        await frames(page);
    };
    const fixture = fixtureNode(reference, 2, {
        kind: "vector", name: "Branching vector network", x: 40, y: 40, width: 200, height: 160,
        corner_radius: 0,
        fills: [{ paint: { type: "solid", r: 239, g: 68, b: 68, a: 255 }, opacity: 1, visible: true }],
        strokes: [{ paint: { type: "solid", r: 24, g: 24, b: 24, a: 255 }, width: 3, opacity: 1, visible: true,
            align: "center", cap: "round", join: "round", dash: [] }],
        path: { fill_rule: "non_zero", segments: [
            { type: "move_to", point: { x: 0, y: 0 } },
            { type: "line_to", point: { x: 200, y: 0 } },
            { type: "line_to", point: { x: 100, y: 160 } },
            { type: "close" },
        ] },
    });
    await importFixture(page, isolatedDocument(reference, [fixture], "Vector graph authoring"));
    // Leave room for a branch outside the original fitted bounds.
    for (let i = 0; i < 3; i++) await command("zoom-out");
    await clickWorld(page, 140, 100);
    await command("vector-network");
    const converted = await node();
    assert.equal((await state(page)).vectorNode, 2);
    assert.equal(converted.path.segments.length, 0);
    assert.equal(Object.keys(converted.vector_network.vertices).length, 3);
    assert.equal(Object.keys(converted.vector_network.edges).length, 3);
    assert.equal(Object.keys(converted.vector_network.regions).length, 1);
    const image = PNG.sync.read(await captureNativeScreenshot(mode + "-vector-network-fill", {
        controls: ["CanvasHost", "DoneVectorButton"],
    })), p = screen(await state(page), 140, 100);
    let red = 0;
    for (let y = Math.round(p.y) - 3; y <= Math.round(p.y) + 3; y++) {
        for (let x = Math.round(p.x) - 3; x <= Math.round(p.x) + 3; x++) {
            const offset = (y * image.width + x) * 4;
            if (image.data[offset] > 200 && image.data[offset + 1] < 100 && image.data[offset + 2] < 100) red++;
        }
    }
    assert.ok(red >= 35, "Converted network fill must be drawn by the real renderer");
    await command("undo");
    assert.equal((await node()).vector_network, undefined);
    await command("redo");
    assert.deepEqual(await node(), converted);
    // Undo exits editing when the legacy path returns; explicitly re-enter.
    await command("edit-vector");
    await clickNative(page, "NetworkPenButton", "InspectorViewport");
    await clickWorld(page, 40, 40);
    await clickWorld(page, 0, 160);
    await frames(page);
    const branched = await node();
    const originalVertexIds = new Set(Object.keys(converted.vector_network.vertices));
    const branch = Object.keys(branched.vector_network.vertices).find((id) => !originalVertexIds.has(id));
    assert.ok(branch);
    assert.equal(Object.keys(branched.vector_network.edges).length, 4);
    assert.deepEqual(branched.vector_network.regions, converted.vector_network.regions, "Adding a branch must preserve the explicit fill boundary");
    const branchEdge = Object.entries(branched.vector_network.edges).find(([, edge]) => String(edge.start) === branch || String(edge.end) === branch)?.[0];
    assert.ok(branchEdge);
    await clickNative(page, "VectorSelectButton", "InspectorViewport");
    await drag([0, 160], [-40, 200]);
    const moved = await node(), vertex = moved.vector_network.vertices[branch].position;
    assert.ok(Math.abs(moved.x + vertex.x + 40) < 0.7 && Math.abs(moved.y + vertex.y - 200) < 0.7);
    assert.deepEqual(Object.keys(moved.vector_network.vertices), Object.keys(branched.vector_network.vertices));
    assert.deepEqual(Object.keys(moved.vector_network.edges), Object.keys(branched.vector_network.edges));
    await command("undo");
    assert.deepEqual(await node(), branched, "A multi-event network drag must undo as one exact document edit");
    await command("redo");
    assert.deepEqual(await node(), moved);
    await clickWorld(page, 0, 120, { clickCount: 2 });
    await frames(page);
    const split = await node();
    assert.equal(Object.keys(split.vector_network.vertices).length, 5);
    assert.equal(Object.keys(split.vector_network.edges).length, 5);
    assert.deepEqual(split.vector_network.regions, moved.vector_network.regions);
    await command("undo");
    assert.deepEqual(await node(), moved);
    check("Native graph conversion, branching, stable-ID vertex dragging and edge subdivision preserve regions and exact one-step history");

    await clickNative(page, "VectorBendButton", "InspectorViewport");
    await drag([0, 120], [24, 132]);
    const bent = await node(), graph = bent.vector_network, edge = graph.edges[branchEdge];
    assert.equal(edge.curve.type, "cubic");
    const a = graph.vertices[edge.start].position, b = graph.vertices[edge.end].position;
    const c = edge.curve.control1, d = edge.curve.control2;
    const midpoint = { x: (a.x + 3 * c.x + 3 * d.x + b.x) / 8 + bent.x, y: (a.y + 3 * c.y + 3 * d.y + b.y) / 8 + bent.y };
    assert.ok(Math.abs(midpoint.x - 24) < 1 && Math.abs(midpoint.y - 132) < 1, "The curve must pass through the native bend target");
    await clickNative(page, "VectorEraseButton", "InspectorViewport");
    await clickWorld(page, midpoint.x, midpoint.y);
    await frames(page);
    const erased = await node();
    assert.equal(erased.vector_network.edges[branchEdge], undefined);
    assert.deepEqual(erased.vector_network.regions, bent.vector_network.regions);
    await command("undo");
    assert.deepEqual(await node(), bent);
    await command("redo");
    assert.deepEqual(await node(), erased);
    await captureNativeScreenshot(mode + "-vector-network", {
        controls: ["CanvasHost", "VectorEraseButton"],
    });
    const json = await documentState(page);
    await importFixture(page, json);
    assert.deepEqual(await documentState(page), json, "Stable graph IDs and boundaries must survive a real browser save/import round-trip");
    check("Native network Bend and Erase edit authoritative cubic edges, retain unrelated fill regions and serialize stable topology");
}
