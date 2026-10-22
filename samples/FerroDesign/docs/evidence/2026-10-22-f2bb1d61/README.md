# 2026-10-22 `f2bb1d61` browser and native evidence

This directory records two failed full browser attempts, the intervening successful focused diagnostic, and the completed native CI gate for the keyboard-selection test checkpoint. In the latest unchanged full attempt, Software2D passed **141 checks and all 28 scenarios**; WebGL2 passed **27 checks** before a component-control readiness failure. The focused diagnostic does not replace a complete two-mode gate. The independent exact-source CI browser job was still running at **15:40 UTC**. [Validation and measurements](../../VALIDATION.md) contains the wider continuation history and implementation limits.

## Source and runtime identity

| Input | Exact identity |
| --- | --- |
| Original browser harness | [`f2bb1d618843bf777b76859d2e06bf361bef009b`](https://github.com/wieslawsoltes/FerroUI/commit/f2bb1d618843bf777b76859d2e06bf361bef009b), tree `054e0d03d4d49f327e30e62f22ef78609618a5d2` |
| Recovered CI distribution | Built at [`3a0bf1f13c6ede9abe5360bc3691085a3bc981f0`](https://github.com/wieslawsoltes/FerroUI/commit/3a0bf1f13c6ede9abe5360bc3691085a3bc981f0), from GitHub artifact `12427798904` |
| Actual WASM payload | **51,205,801 bytes**, SHA-256 `7880a628e7117ab88f896f69baa9c9ccedcdc33792302a01f653b4465e1499b9` |
| Documentation checkpoint | [`177f92cdb743d1897aa65a8c8c92e5b4b844e521`](https://github.com/wieslawsoltes/FerroUI/commit/177f92cdb743d1897aa65a8c8c92e5b4b844e521), containing documentation and evidence changes only |
| Browser tools | Chromium **156.0.8078.4**, Playwright **1.64.1**, Node **24.19.0** |
| Rendering | FerroUI Vello; requested Software2D and WebGL2 surfaces. WebGL2 reports ANGLE/Vulkan/SwiftShader |

[validation-source.json](validation-source.json) records the distribution, lockfile, dependency graph, module checks and individual harness/runtime hashes. The recovered artifact ZIP matched GitHub's recorded size and SHA-256 and passed its CRC check. This runtime differs from the original local binary used by the [preceding continuation attempts](../2026-10-22-f36dabf7/README.md); those runtime identities are not interchangeable.

[input-equivalence.json](input-equivalence.json) records the five browser-test files changed from build source `3a0bf1f1` to harness `f2bb1d61`, with no application or build-input changes. It also verifies 31 harness/package files against both `f2bb1d61` and documentation checkpoint `177f92cd`. The documentation commit did not replace the frozen test inputs or rebuild the application.

## Original full browser attempt

The original, uninstrumented harness requested both rendering modes and all scenarios (`full: true`). Its overall result was **failed**.

| Mode | Observed result | Shared native captures |
| --- | --- | --- |
| Software2D | **141 checks passed** | All **46** captures passed on their first attempt |
| WebGL2 | **21 checks passed**, then native variable authoring timed out while waiting for alias 102 to disappear | All **6** captures reached before the failure passed on their first attempt |

The [failed-run summary](failed-full-summary.json) indexes the unchanged report. [failed-full-browser-report.zip](failed-full-browser-report.zip) preserves the original report, browser log, input/source manifests, failure document, selected trace JPEGs and diagnostic records, with a per-member SHA-256 manifest. It is a selected evidence archive: the original large Playwright trace ZIP is not included. The extracted JPEGs and derived timing records identify their original trace events. The separate 15-second failure screenshot also timed out, so there is no failure PNG to present as a successful capture.

Neither mode reported a page exception. The Software2D report retains one resource-404 console message; the WebGL2 report has no console errors. A completed Software2D mode within this failed run is not presented as a successful full two-mode gate.

### What the alias-deletion trace establishes

The failing assertion was `!doc.variables[102]` in `authoring-browser.mjs`, after one native **Delete variable** click. Immediately before the click, the dialog showed **Native alias**, its error text was hidden, and the enabled, arranged Delete button contained the actual pointer coordinates. Its native scene had already advanced after selecting that variable.

The [diagnosis](failed-authoring-diagnosis.json) retains these observations:

- The two animation-frame callbacks following the click took **2.127 seconds**.
- The configured **10-second** document predicate wait returned its timeout after **34.294 seconds** of trace time.
- A trace image received about **391 ms after the wait began** already showed two remaining variables, **Accent** and **Alias**, with the dialog editing Accent. Its mapped swap timestamp preceded the wait's start. The native handler commits deletion before rebuilding that dialog, so this rendered state supports successful deletion before the nominal deadline.
- The subsequent failure-state evaluation took **118.002 seconds**. Its exported document confirmed alias 102 absent, the selected layer's binding cleared, revision **17 → 18**, and history **9 → 10**.

The original trace did not record the predicate's invocation times or returned values. These observations establish substantial scheduling or protocol delays, but do not identify the responsible process or prove a specific application, browser or transport cause. Playwright invokes the first predicate immediately when its evaluation reaches the page, before scheduling later animation-frame polls. Changing only the polling mode was therefore not treated as a demonstrated correction.

## Focused diagnostic with read-only instrumentation

The subsequent diagnostic ran **only WebGL2 native-authoring plus the harness's common gates**, with `full: false`. It passed **21 checks**, including both native-authoring cases, in approximately **175 seconds**. All **3** shared native captures passed on their first attempt.

This run used a separate copy of the `f2bb1d61` harness. Only that copy of `tests/authoring-browser.mjs` gained timestamp and result logging around the existing document predicates. The application, original harness, single-click input path, **10-second deadline**, default animation-frame polling, document assertions, history assertions and pixel assertions were unchanged. The diagnostic source manifest records the original and instrumented file hashes and confirms every other test hash remained equal.

[focused-authoring-summary.json](focused-authoring-summary.json) records all nine document waits, and the [terminal summary](focused-authoring-terminal-summary.json) records the successful exit and exact diagnostic scope. Each predicate ran once and returned true on its first evaluation. For the alias deletion, the page evaluated the predicate at **15:24:44.418–.419 UTC**; the complete Node-side wait took **6 ms**. The intended deletion, binding removal and exact undo/redo assertions all passed. The report retains one resource-404 console message and no page exceptions.

[focused-authoring-diagnostic.zip](focused-authoring-diagnostic.zip) contains the raw report, browser log, predicate timestamps, resource samples, source manifest, instrumentation diff, runner/terminal records, capture diagnostics and the actual variable-dialog PNG. Its per-member manifest identifies the archived bytes. The original large focused trace ZIP is not included. The added logging can itself affect scheduling. This successful diagnostic supports the tested behavior under those conditions; it does not explain the earlier stall or establish complete browser success. No application or published test change was made in response to this failure.

## Resource observations and their limits

[resource-pressure-observations.json](resource-pressure-observations.json) records cgroup snapshots during cleanup after the failed full attempt. The shared memory limit was **8,589,934,592 bytes**. Between **15:16:41 and 15:17:47 UTC**, cumulative `oom` increased from **16 to 18** and `oom_kill` from **11 to 12**. That delta establishes new events in that interval, but identifies neither the killed process nor the cause of the earlier predicate timeout.

The focused diagnostic collected 37 resource samples. Its observed cgroup memory peak was **8,437,272,576 bytes**; the peak sum of its owned processes' RSS was **1,556,766,720 bytes**. The cgroup `max`, `oom` and `oom_kill` counters did not change during that diagnostic. Cgroup memory includes cache and other processes, while summed process RSS can count shared pages more than once. These figures are not a measurement of the editor's exclusive heap or a demonstrated explanation for the failed full run.

## Exact-source native CI

The native job at source `f2bb1d61` completed successfully at **15:05:07 UTC** in [FerroDesign run 39799180117 / job 119894677519](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117/job/119894677519).

| Native gate | Completed result |
| --- | --- |
| Document core | **394** passed; no failed or ignored tests |
| Native application | **155** passed; no failed or ignored tests |
| Combined core/application count | **549** Rust tests passed |
| Persistence host | **19** passed; no failures, cancellations or skipped tests |
| Active renderer graphs | Vello present and Skia absent for Emscripten, Linux x64, Windows x64, macOS ARM64 and macOS x64 |

[native-ci-summary.json](native-ci-summary.json) records checkout identity, gate excerpts, job status and log digest. [native-ci-log.zip](native-ci-log.zip) preserves the decoded native job log and its summary. This ZIP packages connector-returned log text; it is not the original uploaded workflow artifact. The job also completed the public text-input, compositing, geometry and outline API gates recorded in that log.

These native results belong to the exact `f2bb1d61` checkout. They do not certify a complete browser run or native FerroDesign GUI execution on every dependency-audited platform. Earlier 14-job framework successes retain their own source identities in the [preceding evidence index](../2026-10-22-f36dabf7/README.md).

### Native release measurements

The same successful native job produced the current release benchmark: [schema-5 JSON](native-performance/performance-extracted-log.json), [timestamped supporting step](native-performance/performance-step.log), [extraction provenance](native-performance/provenance.json) and [full measurement summary](native-performance/SUMMARY.md). These files preserve decoded-job-log evidence; they do not claim byte identity with a separately downloaded benchmark artifact.

On 10,000 flat filled rectangles with one selected node, scoped nudge measured **0.019659 ms p50**, compared with **17.284794 ms p50** for the explicit full-document transaction reference, with 48 samples per path in the same release process. The scoped gesture-update distribution measured **0.000751 ms p50** across 288 updates. Document/history, serialization and component ID/history equivalence checks passed. These costs exclude browser input, native control layout, scene drawing, GPU execution and durable-storage completion. Component edits retain full-document history, and the detailed summary reports those production costs separately from isolated no-history operations. The earlier `f36dabf7` benchmark remains preserved under its original source and runner scope.

## Second full browser attempt

The unchanged unfiltered rerun ran from **15:27:24 to 15:39:17 UTC**, using the original, uninstrumented `f2bb1d61` harness and the same recovered `3a0bf1f1` CI runtime. It requested both Software2D and WebGL2 with all scenarios, the original assertions and the existing capture policy. Its overall result was **failed**.

| Mode | Observed result | Shared native captures |
| --- | --- | --- |
| Software2D | **141 checks and all 28 scenarios passed** | All **46** passed on their first attempt |
| WebGL2 | **27 checks passed**; vector-network, native-authoring and typography completed, then component authoring failed at the `ComponentValue2` readiness check | All **7** reached before the failure passed on their first attempt |

[failed-second-full-summary.json](failed-second-full-summary.json) preserves the terminal result and integrity/resource observations. [failed-second-full-browser-report.zip](failed-second-full-browser-report.zip) preserves the report and supporting evidence. All **53** shared native captures passed on their first attempt. Neither mode reported a page error; Software2D retained one resource-404 console message, and WebGL2 had no console errors. Post-run hashes confirm that all **31 source files and 16 runtime files** remained unchanged. The second failure's cause is under investigation; no application or test correction is attributed to this result.

Resource sampling was separate from the original test inputs. Across 143 samples at five-second intervals, the shared cgroup reached its **8,589,934,592-byte** memory limit. `memory.events.max` increased by **1,631,582**, while `oom` and `oom_kill` had zero delta. Peak summed RSS for owned processes was **2,373,701,632 bytes**. These shared-cgroup and summed-RSS observations have the attribution limits described above and do not establish the cause of the readiness failure.

The independent exact-`f2bb1d61` CI browser job in run 39799180117 was still running at **15:40 UTC**. The native job's successful result remains separate. Neither this failed local run nor the focused diagnostic is recorded as a complete browser success.

## Captures and interpretation

The standalone [Software2D initial shell](Software2D-initial.png), [WebGL2 initial shell](WebGL2-initial.png), [Software2D stacked guides](Software2D-stacked-layout-guides.png), [Software2D matching controls](Software2D-native-matching-fill.png) and [Software2D bucket preview](Software2D-bucket-hover-face.png) come from the original full attempt recorded above. They use the recovered CI runtime and are not labeled as images from the second attempt.

These captures show this application's rendered UI. No matched Figma document/reference screenshot was available for a numerical visual-parity comparison. SwiftShader execution is not physical-GPU throughput evidence, and the diagnostic's elapsed times are not frame-rate measurements.
