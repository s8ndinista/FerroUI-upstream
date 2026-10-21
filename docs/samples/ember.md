# Ember — Hacker News for FerroUI

Ember is an original, responsive Hacker News reader built with FerroUI controls,
compiled XAML, the WinUI 3 theme, and composition animations. This document starts
the implementation branch and is updated as each part becomes verifiable.

## Product contract

- Six HN feeds; ordered pagination, refresh, full-text search and filters.
- Adaptive navigation and reading panes, generous touch targets, keyboard access.
- Rich, safe HN HTML; nested discussion, branch collapse, profile inspection.
- Saved stories, history, read/hidden state, mute rules and offline reading.
- Versioned persistent storage: SQLite on native platforms and IndexedDB in browsers.
- System/light/dark/custom appearance, typography, density and reduced motion.
- External article, share and HN account actions with clearly identified handoff.

The documented HN API is read-only. Voting, replying, sign-in and submission open
HN itself; local saved stories never imply synchronization with an HN account.
Third-party article extraction is not assumed to work across arbitrary origins.

## Architecture

- `samples/Ember.Core`: transport-independent HN models, repository, caching,
  persistent reader state and safe thread traversal.
- `samples/Ember.Content`: sanitized HTML document model and FerroUI rich content.
- `samples/Ember`: shared application, themed controls and responsive reader shell.
- `samples/Ember.Desktop`, `Ember.Browser`, `Ember.Android`, `Ember.iOS`:
  platform entry points and platform services.

All rendered reader content uses FerroUI's retained control tree and drawing APIs.
Networking and persistence stay off the UI thread or use asynchronous browser APIs.
Data errors retain readable cached content and surface a recovery action.

## Validation plan

Run deterministic model/cache/database/parser tests, compile the shared UI and
desktop host, validate browser bindings, render desktop and compact layouts, and
exercise navigation, thread collapse, persistence, search and theme changes.
Platform results and unverified device scenarios will be recorded explicitly.

## Sources

- https://github.com/HackerNews/API
- https://hn.algolia.com/api
- https://www.ycombinator.com/blog/hacker-news-api
