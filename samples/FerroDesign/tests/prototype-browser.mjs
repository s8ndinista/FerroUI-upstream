import assert from "node:assert/strict";
import { resolve } from "node:path";

// The main browser gate supplies its real native-input and document helpers.
// This module never launches a browser or advances a fabricated animation clock.
export async function prototypeMotionRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, moveWorld, frames } = helpers;
    const instant = { kind: "instant", duration_ms: 0, easing: "linear" };
    const rule = (trigger, action, transition = instant) => ({ trigger, action, transition, delay_ms: 0, key: null });
    const waitStopped = () => page.waitForFunction(() => {
        const state = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return state.prototypeAnimation === null;
    }, undefined, { timeout: 10000 });
    const present = async () => {
        assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("present")), true);
        await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 10);
        await frames(page);
    };
    const stop = async () => {
        await page.keyboard.press("Escape");
        await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === null);
    };
    const frame = (id, children, extra = {}) => fixtureNode(reference, id, {
        kind: "frame", name: "Motion frame " + id, x: id === 10 ? 0 : 400, y: 0,
        width: 320, height: 220, clip_content: true, children, ...extra,
    });
    const source = fixtureNode(reference, 11, {
        parent: 10, name: "Animated button", x: 40, y: 60, width: 120, height: 60,
        fills: [{ paint: { type: "solid", r: 255, g: 0, b: 0, a: 255 }, visible: true, opacity: 1 }],
        interactions: [rule("click", { type: "navigate_to", target: 20 }, { kind: "smart_animate", duration_ms: 2000, easing: "linear" })],
    });
    const destination = fixtureNode(reference, 21, {
        parent: 20, name: "Animated button", x: 140, y: 100, width: 140, height: 80,
        fills: structuredClone(source.fills), interactions: [],
    });
    Object.assign(destination.fills[0].paint, { r: 0, g: 0, b: 255, a: 255 });
    const motion = isolatedDocument(reference, [frame(10, [11]), source, frame(20, [21]), destination], "Native clock motion");
    motion.pages[0].prototype_start = 10;
    await importFixture(page, motion);
    const authored = await documentState(page);
    await present();
    const initial = await state(page);
    const point = await moveWorld(page, 100, 90);
    await page.mouse.click(point.x, point.y);
    const midHandle = await page.waitForFunction(() => {
        const state = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        const animation = state.prototypeAnimation;
        return animation && animation.progress > 0.2 && animation.progress < 0.8 ? state : false;
    }, undefined, { polling: "raf", timeout: 10000 });
    const mid = await midHandle.jsonValue();
    const layer = mid.prototypeAnimation.layers.find((node) => node.name === "Animated button");
    assert.ok(layer && layer.x > 40 && layer.x < 140, "The rendered scene must contain an interpolated layer position");
    assert.ok(layer.width > 120 && layer.width < 140, "Smart animate must interpolate size");
    assert.ok(layer.fill.r > 0 && layer.fill.r < 255 && layer.fill.b > 0 && layer.fill.b < 255, "Smart animate must interpolate native solid brush colors");
    assert.ok(mid.renderedFrames > initial.renderedFrames, "The native clock must drive visible render frames");
    assert.equal(mid.prototypeAnimation.animatedNodes, 1);
    await page.screenshot({ path: resolve(output, mode + "-prototype-smart-midpoint.png") });
    await waitStopped();
    assert.equal((await state(page)).presentation, 20);
    assert.deepEqual(await documentState(page), authored);
    await stop();
    assert.deepEqual(await documentState(page), authored);
    check("Native FerroUI Clock renders Smart Animate midpoint position, size and brush color, then restores the exact authored document on exit");

    const scrollButton = fixtureNode(reference, 11, {
        parent: 10, name: "Scroll button", x: 40, y: 40, width: 120, height: 60,
        interactions: [rule("click", { type: "scroll_to", target: 12 }, { kind: "smart_animate", duration_ms: 600, easing: "ease_in_out" })],
    });
    const target = fixtureNode(reference, 12, { parent: 10, name: "Scroll destination", x: 40, y: 600, width: 120, height: 80, interactions: [] });
    const scrolling = isolatedDocument(reference, [frame(10, [11, 12], { prototype_overflow: "vertical" }), scrollButton, target], "Native content scrolling");
    scrolling.pages[0].prototype_start = 10;
    await importFixture(page, scrolling);
    const authoredScroll = await documentState(page);
    await present();
    const camera = await state(page);
    const scrollPoint = await moveWorld(page, 100, 70);
    await page.mouse.click(scrollPoint.x, scrollPoint.y);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).prototypeScrollOffsets?.[10]?.y === 460);
    await waitStopped();
    await moveWorld(page, 100, 170);
    await page.mouse.wheel(0, -120);
    await page.waitForFunction(() => {
        const offset = JSON.parse(globalThis.ferroDesign.ferroDesignState()).prototypeScrollOffsets?.[10]?.y;
        return Number.isFinite(offset) && offset < 460;
    });
    const scrolled = await state(page);
    assert.deepEqual(scrolled.pan, camera.pan, "Wheel scrolling must keep the presentation camera fixed");
    assert.equal(scrolled.zoom, camera.zoom);
    await page.screenshot({ path: resolve(output, mode + "-prototype-scroll.png") });
    await stop();
    assert.deepEqual(await documentState(page), authoredScroll);
    check("ScrollTo and actual native wheel input move clamped frame contents without panning the camera or changing authored nodes");

    const reversible = isolatedDocument(reference, [
        frame(10, [11]),
        fixtureNode(reference, 11, { parent: 10, x: 40, y: 60, width: 120, height: 60, interactions: [rule("while_hover", { type: "navigate_to", target: 20 })] }),
        frame(20, []),
    ], "Reversible native hover");
    reversible.pages[0].prototype_start = 10;
    await importFixture(page, reversible);
    await present();
    const hover = await moveWorld(page, 100, 90);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 20);
    await page.mouse.move(hover.x + 2, hover.y + 2);
    await frames(page);
    assert.equal((await state(page)).presentation, 20, "A new frame's node IDs must not end hover inside the original hit region");
    await page.mouse.move(0, 0);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 10);
    await stop();
    reversible.name = "Reversible native press";
    reversible.nodes[11].interactions[0].trigger = "while_press";
    await importFixture(page, reversible);
    await present();
    await moveWorld(page, 100, 90);
    await page.mouse.down();
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 20);
    await page.mouse.up();
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 10);
    await stop();
    check("Actual pointer hover/exit and press/release restore reversible prototype navigation; hover survives changed destination node IDs");
}
