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
const svgFixtureName = "Browser SVG fixture.svg";
const svgFixture = `<svg xmlns="http://www.w3.org/2000/svg" width="320" height="220" viewBox="0 0 320 220">
  <defs>
    <linearGradient id="qa-gradient" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#ef4444"/>
      <stop offset="1" stop-color="#2563eb"/>
    </linearGradient>
  </defs>
  <rect id="qa-background" width="320" height="220" fill="#f8fafc"/>
  <g id="qa-affine" transform="matrix(1 .15 .2 1 24 24)">
    <rect id="qa-panel" width="180" height="78" rx="8" fill="url(#qa-gradient)"/>
    <path id="qa-curve" d="M12 94 C52 50 114 144 176 96" fill="none" stroke="#10b981" stroke-width="8" stroke-linecap="round"/>
  </g>
  <rect id="qa-chip" x="264" y="24" width="32" height="32" fill="#10b981"/>
  <text id="qa-unicode" x="20" y="194" font-family="Inter" font-size="20" fill="#111827">Orbit · café Ω</text>
</svg>`;
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
function pixel(png, x, y) {
    const offset = (y * png.width + x) * 4;
    return [...png.data.subarray(offset, offset + 4)];
}
function assertColor(actual, expected, label) {
    assert.ok(actual[3] >= 240, label + " must be opaque");
    for (let channel = 0; channel < 3; channel++) {
        assert.ok(Math.abs(actual[channel] - expected[channel]) <= 12,
            label + ": expected " + expected.join(",") + ", got " + actual.join(","));
    }
}
function countPixels(png, bounds, matches) {
    let count = 0;
    for (let y = bounds.top; y < bounds.bottom; y++) {
        for (let x = bounds.left; x < bounds.right; x++) {
            const offset = (y * png.width + x) * 4;
            if (matches(png.data[offset], png.data[offset + 1], png.data[offset + 2], png.data[offset + 3])) count++;
        }
    }
    return count;
}
try {
    browser = await chromium.launch({
        headless: true,
        args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"],
    });
    report.browser = browser.version();
    for (const mode of ["Software2D", "WebGL2"]) {
        const context = await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1, acceptDownloads: true });
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
            await page.waitForFunction(() => globalThis.ferroDesignReady === true || globalThis.ferroDesignErrors?.length > 0, undefined, { timeout: 120000 });
            const startupErrors = await page.evaluate(() => globalThis.ferroDesignErrors || []);
            assert.deepEqual(startupErrors, [], "The application must start without a runtime or module-loading error");
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

            // Import through the same public bridge used by the real file picker.
            const beforeSvg = await state(page);
            assert.equal(await page.evaluate(({ source, name }) => globalThis.ferroDesign.ferroDesignImportFile(source, name),
                { source: svgFixture, name: svgFixtureName }), true, "The SVG fixture must import successfully");
            await page.waitForFunction((before) => {
                const data = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                return data.nodeCount > before.nodeCount && data.selectionCount === 1 && data.renderedFrames > before.renderedFrames;
            }, beforeSvg);
            await frames(page);
            result.svgImport = await state(page);
            assert.equal(result.svgImport.invalidNodes, 0, "Imported SVG must produce no invalid renderer nodes");
            assert.equal(result.svgImport.missingImages, 0, "Imported SVG must not introduce missing resources");
            const svgJson = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            const svgDocument = JSON.parse(svgJson);
            const importedRootId = result.svgImport.selection[0];
            const importedRoot = svgDocument.nodes[String(importedRootId)];
            assert.ok(importedRoot, "Selected import root must exist in the serialized document");
            assert.equal(importedRoot.kind, "frame");
            assert.equal(importedRoot.name, svgFixtureName);
            assert.equal(importedRoot.width, 320);
            assert.equal(importedRoot.height, 220);
            assert.equal(importedRoot.parent, result.svgImport.activePage);
            assert.ok(svgDocument.pages.find((item) => item.id === result.svgImport.activePage).children.includes(importedRootId));
            const findSvgNode = (id) => Object.values(svgDocument.nodes).find((node) => node.metadata?.svg_id === id);
            assert.ok(findSvgNode("qa-affine")?.affine_transform, "Group shear must remain an editable affine transform");
            assert.equal(findSvgNode("qa-panel")?.fills[0]?.paint.type, "linear_gradient");
            assert.ok(findSvgNode("qa-curve")?.path.segments.some((segment) => segment.type === "cubic_to"), "Curves must remain editable path segments");
            assert.equal(findSvgNode("qa-unicode")?.text, "Orbit · café Ω");
            await canvas.screenshot({ path: resolve(output, mode + "-svg.png") });
            check("SVG file import selects an editable frame with affine groups, gradients, curves, and Unicode text");

            // Capture the actual application download. This exercises the FerroUI
            // offscreen render target and PNG encoder, independently of screenshots.
            const [download, exported] = await Promise.all([
                page.waitForEvent("download", { timeout: 60000 }),
                page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("export-png")),
            ]);
            assert.equal(exported, true, "The application must handle the PNG export command");
            assert.equal(await download.failure(), null, "PNG download must finish successfully");
            assert.match(download.suggestedFilename(), /\.png$/i);
            const pngPath = resolve(output, mode + "-export.png");
            await download.saveAs(pngPath);
            const pngBytes = await readFile(pngPath);
            assert.deepEqual(pngBytes.subarray(0, 8), Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), "Download must be a real PNG file");
            const png = PNG.sync.read(pngBytes);
            assert.equal(png.width, 320, "PNG width must match the selected SVG frame");
            assert.equal(png.height, 220, "PNG height must match the selected SVG frame");
            const background = pixel(png, 8, 8), chip = pixel(png, 280, 40);
            assertColor(background, [248, 250, 252], "Exported artboard background");
            assertColor(chip, [16, 185, 129], "Exported solid green shape");
            const red = pixel(png, 45, 50), blue = pixel(png, 205, 113);
            assert.ok(red[3] > 240 && red[0] > 160 && red[0] > red[2] + 50, "Gradient start must render red");
            assert.ok(blue[3] > 240 && blue[2] > 160 && blue[2] > blue[0] + 50,
                "Gradient end must render blue at its affine-transformed position");
            const curvePixels = countPixels(png, { left: 30, top: 90, right: 245, bottom: 170 },
                (r, g, b, a) => a > 200 && r < 100 && g > 130 && b > 40 && b < 180 && g > b + 20);
            assert.ok(curvePixels > 100, "The exported curved stroke must contain visible green pixels");
            const textPixels = countPixels(png, { left: 15, top: 165, right: 305, bottom: 210 },
                (r, g, b, a) => a > 200 && r < 90 && g < 100 && b < 130);
            assert.ok(textPixels > 60, "The exported Unicode text must contain rendered glyph pixels");
            result.pngExport = { filename: download.suggestedFilename(), bytes: pngBytes.length, ...colors(pngBytes), background, chip, red, blue, curvePixels, textPixels };
            assert.ok(result.pngExport.colors >= 32, "Export must retain the gradient's color variation");
            check("PNG download has exact artboard dimensions and verified background, affine gradient, curved stroke, and text pixels");

            assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), svgJson), true);
            await frames(page);
            const reopenedSvgJson = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            assert.deepEqual(JSON.parse(reopenedSvgJson), svgDocument, "Saving and reopening must preserve every imported SVG field");
            const afterSvgRoundTrip = await state(page);
            assert.equal(afterSvgRoundTrip.invalidNodes, 0, "Reopened SVG must still contain valid renderer nodes");
            assert.equal(afterSvgRoundTrip.missingImages, 0, "Reopened SVG must not introduce missing resources");
            check("JSON round-trip preserves the complete imported SVG hierarchy and visual properties");
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
    const body = report.results.map((item) => "<section><h2>" + escape(item.requestedMode + ": " + item.status) + "</h2><p>Actual surface: " + escape(item.surface?.kind || "not initialized") + "</p><ul>" + item.checks.map((check) => "<li>" + escape(check) + "</li>").join("") + "</ul><img src='" + item.requestedMode + (item.status === "passed" ? "-edited.png" : "-failure.png") + "' alt='Actual browser screenshot'>" + (item.svgImport ? "<h3>Imported SVG in the application</h3><img src='" + item.requestedMode + "-svg.png' alt='SVG imported and rendered by FerroUI'>" : "") + (item.pngExport ? "<h3>Actual PNG download</h3><img class='export' src='" + item.requestedMode + "-export.png' alt='PNG exported by the application'>" : "") + "<pre>" + escape(item.error || JSON.stringify(item, null, 2)) + "</pre></section>").join("");
    await writeFile(resolve(output, "index.html"), "<!doctype html><meta charset='utf-8'><title>FerroDesign browser validation</title><style>body{font:15px system-ui;margin:32px;background:#f6f6f6;color:#222}section{background:white;padding:24px;margin:24px 0;border-radius:12px}img{width:100%;border:1px solid #ddd}pre{overflow:auto;font-size:12px;white-space:pre-wrap}</style><h1>FerroDesign browser validation</h1><p>Chromium " + escape(report.browser) + ". Requested WebGL2 may fall back to CPU; the actual surface above records what was exercised. CI uses a software GPU and is not a hardware performance measurement.</p>" + body);
    console.log(JSON.stringify({ passed: report.passed, browser: report.browser, results: report.results.map(({requestedMode,status,surface,checks}) => ({requestedMode,status,surface,checks})) }, null, 2));
}
if (failed) process.exitCode = 1;
