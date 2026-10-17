import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile, writeFile, mkdir, stat } from "node:fs/promises";
import { resolve, dirname, extname, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { PNG } from "pngjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dist = resolve(process.env.FERRODESIGN_DIST || resolve(root, "dist"));
const output = resolve(process.env.FERRODESIGN_REPORT || resolve(root, "browser-report"));
await mkdir(output, { recursive: true });
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json", ".png": "image/png", ".svg": "image/svg+xml" };
const server = createServer(async (request, response) => {
    try {
        const pathname = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
        let file = resolve(dist, "." + pathname);
        if (file !== dist && !file.startsWith(dist + sep)) { response.writeHead(403).end(); return; }
        if ((await stat(file)).isDirectory()) file = resolve(file, "index.html");
        response.writeHead(200, { "content-type": types[extname(file)] || "application/octet-stream", "cache-control": "no-store" });
        response.end(await readFile(file));
    } catch { response.writeHead(404).end("Not found"); }
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const base = "http://127.0.0.1:" + server.address().port;
const report = { created: new Date().toISOString(), browser: "", results: [] };
let browser;
let failed = false;
const state = (page) => page.evaluate(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()));
const frames = (page) => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
function colors(bytes) {
    const png = PNG.sync.read(bytes), found = new Set();
    for (let i = 0; i < png.data.length; i += 4 * 11) {
        found.add((png.data[i] >> 3) + "," + (png.data[i + 1] >> 3) + "," + (png.data[i + 2] >> 3));
    }
    return { width: png.width, height: png.height, colors: found.size };
}
function changedPixels(before, after) {
    const a = PNG.sync.read(before), b = PNG.sync.read(after);
    assert.equal(a.width, b.width); assert.equal(a.height, b.height);
    let changed = 0;
    for (let i = 0; i < a.data.length; i += 4) {
        if (Math.abs(a.data[i] - b.data[i]) + Math.abs(a.data[i + 1] - b.data[i + 1]) + Math.abs(a.data[i + 2] - b.data[i + 2]) > 24) changed++;
    }
    return changed;
}
try {
    browser = await chromium.launch({
        headless: true,
        args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"],
    });
    report.browser = browser.version();
    for (const mode of ["Software2D", "WebGL2"]) {
        const context = await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 });
        await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
        const page = await context.newPage();
        const result = { requestedMode: mode, status: "running", pageErrors: [], consoleErrors: [], checks: [] };
        report.results.push(result);
        page.on("pageerror", (error) => result.pageErrors.push(error.stack || String(error)));
        page.on("console", (message) => { if (message.type() === "error") result.consoleErrors.push(message.text()); });
        const check = (name) => result.checks.push(name);
        try {
            const started = performance.now();
            await page.goto(base + "/?Renderer=Vello&RenderingMode=" + mode, { waitUntil: "domcontentloaded", timeout: 120000 });
            await page.waitForFunction(() => globalThis.ferroDesignReady === true, undefined, { timeout: 120000 });
            await page.waitForFunction(() => {
                const data = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                return data && data.nodeCount > 0 && data.renderedFrames > 0;
            }, undefined, { timeout: 120000 });
            await page.locator(".ferroui-splash").waitFor({ state: "hidden", timeout: 120000 });
            const canvas = page.locator("canvas.ferroui-canvas").first();
            await canvas.waitFor({ state: "visible" });
            await frames(page);
            result.firstFrameMs = Math.round(performance.now() - started);
            result.initial = await state(page);
            assert.equal(result.initial.renderer, "FerroUI Vello");
            assert.ok(result.initial.pageCount > 0);
            const baseline = await canvas.screenshot({ path: resolve(output, mode + "-initial.png") });
            result.pixels = colors(baseline);
            assert.ok(result.pixels.width >= 1000 && result.pixels.height >= 600, "Full application canvas must be rendered");
            assert.ok(result.pixels.colors >= 24, "The canvas must contain actual application pixels");
            result.surface = await page.evaluate(() => {
                const canvas = document.querySelector("canvas.ferroui-canvas");
                const gl = canvas.getContext("webgl2");
                if (!gl) return { kind: canvas.getContext("2d") ? "Software2D" : "unknown" };
                const debug = gl.getExtension("WEBGL_debug_renderer_info");
                return { kind: "WebGL2", device: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : "unreported" };
            });
            assert.notEqual(result.surface.kind, "unknown");
            if (mode === "Software2D") assert.equal(result.surface.kind, "Software2D");
            check("Native FerroUI canvas contains rendered UI and document pixels");

            // Focus the document viewport through a real pointer event, then invoke its keyboard shortcut.
            await page.mouse.click(720, 630);
            await page.keyboard.press("r");
            await page.mouse.move(720, 630);
            await page.mouse.down();
            await page.mouse.move(900, 770, { steps: 12 });
            await page.mouse.up();
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count + 1, result.initial.nodeCount);
            result.afterDraw = await state(page);
            assert.equal(result.afterDraw.selectionCount, 1, "The newly drawn rectangle must be selected");
            await frames(page);
            const drawn = await canvas.screenshot({ path: resolve(output, mode + "-edited.png") });
            result.changedPixels = changedPixels(baseline, drawn);
            assert.ok(result.changedPixels > 200, "Pointer drawing must change actual canvas pixels");
            check("R shortcut and pointer drag create and render a selected rectangle");

            await page.keyboard.press("Control+z");
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count, result.initial.nodeCount);
            check("Keyboard undo removes the pointer-created node");
            await page.keyboard.press("Control+Shift+z");
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count + 1, result.initial.nodeCount);
            check("Keyboard redo restores the node");

            const saved = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            assert.doesNotThrow(() => JSON.parse(saved));
            await page.keyboard.press("Delete");
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count, result.initial.nodeCount);
            check("Delete keyboard action removes the selection");
            assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), saved), true);
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count + 1, result.initial.nodeCount);
            const restored = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            assert.deepEqual(JSON.parse(restored), JSON.parse(saved), "Document import/export must preserve every serialized field");
            check("Document JSON round-trip restores the complete edited document");
            result.final = await state(page);
            assert.deepEqual(result.pageErrors, [], "No uncaught browser exceptions");
            const panics = await page.evaluate(() => ({
                startup: globalThis.ferroDesignErrors || [],
                render: globalThis.ferroDesignRenderPanics?.() || [],
            }));
            assert.deepEqual(panics, { startup: [], render: [] }, "No startup or rendering panics");
            assert.ok(!result.consoleErrors.some((line) => /panicked|unreachable|RuntimeError|TypeError|ReferenceError|RangeError/i.test(line)), "No runtime errors in the browser console");
            result.status = "passed";
        } catch (error) {
            failed = true;
            result.status = "failed";
            result.error = error.stack || String(error);
            await page.screenshot({ path: resolve(output, mode + "-failure.png"), timeout: 15000 }).catch(() => {});
            console.error(mode + ": " + result.error);
        } finally {
            await context.tracing.stop({ path: resolve(output, mode + "-trace.zip") }).catch(() => {});
            await context.close();
        }
    }
} catch (error) {
    failed = true;
    report.error = error.stack || String(error);
    console.error(error);
} finally {
    await browser?.close();
    await new Promise((done) => server.close(done));
    report.passed = !failed;
    await writeFile(resolve(output, "report.json"), JSON.stringify(report, null, 2));
    const escape = (text) => String(text).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
    const body = report.results.map((item) => "<section><h2>" + escape(item.requestedMode + ": " + item.status) + "</h2><p>Actual surface: " + escape(item.surface?.kind || "not initialized") + "</p><ul>" + item.checks.map((check) => "<li>" + escape(check) + "</li>").join("") + "</ul><img src='" + item.requestedMode + (item.status === "passed" ? "-edited.png" : "-failure.png") + "' alt='Actual browser screenshot'><pre>" + escape(item.error || JSON.stringify(item, null, 2)) + "</pre></section>").join("");
    await writeFile(resolve(output, "index.html"), "<!doctype html><meta charset='utf-8'><title>FerroDesign browser validation</title><style>body{font:15px system-ui;margin:32px;background:#f6f6f6;color:#222}section{background:white;padding:24px;margin:24px 0;border-radius:12px}img{width:100%;border:1px solid #ddd}pre{overflow:auto;font-size:12px;white-space:pre-wrap}</style><h1>FerroDesign browser validation</h1><p>Chromium " + escape(report.browser) + ". Requested WebGL2 may fall back to CPU; the actual surface above records what was exercised. CI uses a software GPU and is not a hardware performance measurement.</p>" + body);
    console.log(JSON.stringify({ passed: report.passed, browser: report.browser, results: report.results.map(({requestedMode,status,surface,checks}) => ({requestedMode,status,surface,checks})) }, null, 2));
}
if (failed) process.exitCode = 1;
