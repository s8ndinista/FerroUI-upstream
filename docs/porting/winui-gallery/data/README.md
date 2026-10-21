# WinUI / Gallery parity data

See [`scripts/winui-gallery/README.md`](../../../../scripts/winui-gallery/README.md)
for commands, schemas, source boundaries and the distinction between declaration
candidates and behavioral evidence. Start with [the generated report](REPORT.md).

`sources.lock.json`, `api-mappings.json` and `api-evidence.json` are reviewed inputs. The upstream
snapshots are regenerated only from the pinned, clean repositories. All remaining
reports are regenerated from those snapshots plus current FerroUI sources and the
Gallery sample's curated port-status manifest. No file in this directory asserts
that a name match is functional or visual parity.
