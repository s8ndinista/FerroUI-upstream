import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";
import { inlineInputState, startInputObservation, stopInputObservation } from "./input-observations.mjs";

const page = { evaluate: async (callback, args) => callback(args) };
const originals = new Map();
beforeEach(() => {
    for (const key of ["document", "Element", "ferroDesign", "PerformanceObserver"])
        originals.set(key, Object.getOwnPropertyDescriptor(globalThis, key));
    globalThis.document = new EventTarget();
    globalThis.Element = class {};
    globalThis.ferroDesign = { ferroDesignState() { throw new Error("Input observers must not poll native state"); } };
    globalThis.PerformanceObserver = undefined;
});
afterEach(async () => {
    await stopInputObservation(page);
    for (const [key, descriptor] of originals) {
        if (descriptor) Object.defineProperty(globalThis, key, descriptor);
        else delete globalThis[key];
    }
});

function press() {
    const event = new Event("pointerdown", { cancelable: true });
    Object.assign(event, { button: 0, buttons: 1, clientX: 12, clientY: 34, detail: 0, pointerId: 1, pointerType: "mouse" });
    document.dispatchEvent(event);
    return event;
}

test("input observation remains passive, labels synthetic input honestly, and does not call the native bridge", async () => {
    await startInputObservation(page);
    const event = press();
    const observation = await stopInputObservation(page);
    assert.equal(event.defaultPrevented, false);
    assert.deepEqual(observation.events.map(event => event.phase), ["capture", "bubble"]);
    assert.ok(observation.events.every(event => event.trusted === false));
    assert.ok(observation.events.every(event => event.eventTimeMs > 0 && event.observedAtMs >= event.eventTimeMs));
    assert.equal(observation.longTaskSupported, false);
    assert.equal(observation.droppedEvents, 0);
    assert.equal(globalThis.__ferroDesignInputObservation, undefined);
});

test("input evidence is bounded and records overflow instead of silently discarding it", async () => {
    await startInputObservation(page, ["pointerdown"], 4);
    for (let index = 0; index < 10; index++) press();
    const observation = await stopInputObservation(page);
    assert.equal(observation.events.length, 4);
    assert.equal(observation.droppedEvents, 16);
    press();
    assert.equal(observation.events.length, 4, "Stopping must remove both listeners");
});

test("nested observers fail explicitly and leave the first observation intact", async () => {
    await startInputObservation(page);
    await assert.rejects(startInputObservation(page), /already active/);
    press();
    assert.equal((await stopInputObservation(page)).events.length, 2);
    assert.equal(await stopInputObservation(page), null);
});

test("inline evidence retains native pointer facts and excludes document and typography payloads", () => {
    const pointerPresses = { serial: 2, entries: [{ timestamp: 1234, clickCount: 2 }] };
    const result = inlineInputState({ renderedFrames: 3, sceneFrames: 4, selection: [2], editingText: 2,
        pointerPresses, document: "not part of the observation", chrome: {
            InlineTextBox: { visible: true, focused: true, width: 100, height: 30, typography: { runs: ["not copied"] } },
        } });
    assert.equal(result.pointerPresses, pointerPresses);
    assert.equal(result.chrome.InlineTextBox.focused, true);
    assert.equal(result.chrome.InlineTextBox.typography, undefined);
    assert.equal(result.document, undefined);
});
