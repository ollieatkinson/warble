# Refactor Audit

Audit date: 2026-03-11

Scope:
- Reviewed app source, build config, and Tauri crate sources.
- Excluded generated or vendor directories: `dist/`, `node_modules/`, `src-tauri/target/`, `src-tauri/icons/`, `src-tauri/gen/`.

## File Inventory

| File | Single responsibility | Filename/placement | Notes |
| --- | --- | --- | --- |
| `README.md` | Template-level project introduction. | Misleading for this app; root is right. | Still template content, not product documentation. |
| `package.json` | Frontend package and script manifest. | Correct. | No issues beyond missing lint/test scripts. |
| `vite.config.ts` | Vite config for Tauri dev/build. | Correct. | Small and focused. |
| `tsconfig.json` | TS compiler config for app source. | Correct. | Small and focused. |
| `tsconfig.node.json` | TS compiler config for Node-side files. | Correct. | Small and focused. |
| `index.html` | Frontend HTML shell. | Correct location. | Title/favicon still template defaults. |
| `src/main.tsx` | React bootstrap. | Correct. | Small and focused. |
| `src/App.tsx` | Choose between main window and indicator window. | Name is right after refactor. | Was a god-file before refactor. |
| `src/ControlApp.tsx` | Compose the main settings/workspace UI. | Correct. | Extracted from previous `App.tsx`. |
| `src/constants.ts` | UI constants and choice lists. | Correct. | Now also hosts media extensions and shared UI timing constants. |
| `src/types.ts` | Shared frontend domain and snapshot types. | Correct. | Large but still cohesive. |
| `src/lib/tauriApi.ts` | Typed frontend IPC wrappers. | Correct. | Added to remove scattered `invoke()` calls. |
| `src/lib/controlAppModel.ts` | Pure view-model derivation for the control UI. | Correct. | Added to separate data shaping from rendering. |
| `src/lib/modelCatalog.ts` | Model catalog data plus model-row derivation. | Mostly correct. | Large but cohesive; data-heavy file, not a god-file. |
| `src/lib/utils.ts` | Formatting, shortcut capture, filtering, and small transforms. | Borderline broad but acceptable in `lib/`. | Utility density is high; worth watching if it grows. |
| `src/hooks/useSnapshotState.ts` | Subscribe to snapshot state from Tauri. | Correct. | Focused and reusable. |
| `src/hooks/useButtonFeedback.ts` | Manage transient button feedback state/timers. | Correct. | Added to remove duplicated timer logic. |
| `src/hooks/useControlApp.ts` | Main-window controller state and actions. | Correct. | Added to separate orchestration from JSX. |
| `src/components/common.tsx` | Shared UI primitives used across sections. | Filename is vague but acceptable. | Large, but responsibility is still "reusable primitives". |
| `src/components/icons.tsx` | Shared SVG icon components. | Correct. | Focused. |
| `src/components/TranscriptionPill.tsx` | Indicator/pill rendering and indicator measurement bridge. | Placement is acceptable. | Broad component file, but responsibilities are tightly related. |
| `src/sections/AboutSection.tsx` | Static product/about cards. | Correct. | Focused. |
| `src/sections/CleanupSection.tsx` | Cleanup settings UI. | Correct. | Focused. |
| `src/sections/HistorySection.tsx` | History search, retention, and item actions UI. | Correct. | Focused. |
| `src/sections/InputsSection.tsx` | Input source selection and preview UI. | Correct. | Focused. |
| `src/sections/InterfaceSection.tsx` | Overlay/interface settings UI. | Correct. | Focused. |
| `src/sections/KeybindingsSection.tsx` | Shortcut and auto-paste settings UI. | Correct. | Focused. |
| `src/sections/ModelsSection.tsx` | Model selection, download, and status UI. | Correct. | Large but still one domain. |
| `src/sections/OverviewSection.tsx` | Dashboard summary UI. | Correct. | Focused. |
| `src/sections/Sidebar.tsx` | Sidebar navigation UI. | Correct. | Focused. |
| `src/styles.css` | Global app styling. | Correct. | Large monolithic stylesheet; acceptable for now but a future split candidate. |
| `src-tauri/build.rs` | Copy Windows ONNX runtime files into build outputs. | Correct. | Focused. |
| `src-tauri/src/main.rs` | Tauri binary bootstrap. | Correct. | Small and focused. |
| `src-tauri/src/lib.rs` | Tauri app bootstrap and orchestration glue. | Correct location, but was a god-file. | Improved by extracting transcript/overlay helpers, but still the main orchestration hub. |
| `src-tauri/src/constants.rs` | Tauri constants/config values. | Correct. | Already acting as a config module. |
| `src-tauri/src/media.rs` | Media-file path validation and audio decode. | Correct. | Focused. |
| `src-tauri/src/models.rs` | Model inspection, selection, install/download/remove workflows. | Correct. | Broad but domain-cohesive. |
| `src-tauri/src/overlay.rs` | Overlay sizing, metering, and indicator window updates. | Correct. | Added to separate overlay logic from bootstrap. |
| `src-tauri/src/parakeet.rs` | Parakeet runtime wrappers and resampling helpers. | Correct. | Focused. |
| `src-tauri/src/platform.rs` | OS-specific caret and paste integrations. | Correct. | Focused. |
| `src-tauri/src/runtime.rs` | ONNX Runtime initialization and availability checks. | Correct. | Focused. |
| `src-tauri/src/state.rs` | Tauri app/domain state types. | Correct. | Improved by removing dependency on crate-root helpers. |
| `src-tauri/src/storage.rs` | Persistence, app-data paths, snapshot emission, and system profiling. | Mixed responsibilities. | Still a split candidate because persistence and hardware detection are separate concerns. |
| `src-tauri/src/streaming_preview.rs` | Streaming preview runtime selection and execution. | Correct. | Focused. |

## Dependency Map

Frontend:
- `App.tsx` now depends on `ControlApp.tsx`, `useSnapshotState.ts`, and `TranscriptionPill.tsx`.
- `ControlApp.tsx` depends on `useControlApp.ts` and section components only.
- `useControlApp.ts` depends on `tauriApi.ts`, `controlAppModel.ts`, and `useButtonFeedback.ts`.
- Section/components depend on `types.ts`, `constants.ts`, `modelCatalog.ts`, and `utils.ts`.
- No frontend circular imports found after extraction.

Rust:
- `lib.rs` depends on `overlay`, `transcript`, `models`, `storage`, `state`, and runtime/platform modules.
- `state.rs` depends on `overlay` and `transcript` defaults instead of crate-root helpers.
- `storage.rs` depends on `overlay::update_indicator_window` instead of `lib.rs`.
- No Rust module cycles remain that require crate-root helper reach-through for state initialization or snapshot persistence.

Layering assessment:
- Frontend now has a clearer `view -> hook/controller -> IPC/data shaping` layering.
- Rust still has orchestration in `lib.rs`, but pure overlay and transcript logic no longer live in the bootstrap file.

## Hardcoded Value Scan

High-value findings before refactor:
- `src/App.tsx`: feedback hold durations like `900`, `1000`, `1200`, `1500` and media extensions array were inline orchestration details.
- `src/sections/HistorySection.tsx`: `2500`ms confirm timeout was inline and unnamed.
- `src/components/TranscriptionPill.tsx`: direct IPC command names were embedded in the component.
- `src-tauri/src/lib.rs`: overlay sizing math, preview constants, and text cleanup logic lived beside bootstrap flow.

What those became:
- Shared frontend constants in `src/constants.ts`.
- Typed IPC wrapper functions in `src/lib/tauriApi.ts`.
- Rust overlay/text logic centralized in `src-tauri/src/overlay.rs` and `src-tauri/src/transcript.rs`.

Notable remaining hardcoded areas:
- `src/styles.css` still contains many visual token values and layout dimensions.
- `src-tauri/src/models.rs` still embeds catalog download specs inline.
- `src-tauri/src/storage.rs` still mixes operational strings and path rules with persistence logic.

## Composition Audit

Key issues found:
- `src/App.tsx` previously mixed local UI state, derived selectors, IPC commands, file dialogs, clipboard, and section composition.
- `src-tauri/src/lib.rs` previously mixed bootstrap, recorder orchestration, overlay math, transcript cleanup, live preview stabilization, and tests.
- `src/components/TranscriptionPill.tsx` mixes pill rendering, demo animation, measurement, and small preview widgets, but those responsibilities are at least in the same feature area.

Refactor targets chosen:
- Extract controller logic from the React top-level component instead of splitting already focused sections.
- Extract pure overlay/text helper logic from the Tauri bootstrap instead of moving recorder/model orchestration prematurely.

## SOLID Check

Single Responsibility:
- Violations were concentrated in `src/App.tsx` and `src-tauri/src/lib.rs`.

Open/Closed:
- Frontend command usage improved: new UI actions extend `tauriApi.ts` and `useControlApp.ts` instead of editing many components.
- Rust still has central setup flow in `lib.rs`, but pure overlay/text behavior is more isolated now.

Liskov Substitution:
- No trait/interface substitution problems stood out in this codebase.

Interface Segregation:
- Frontend improved by replacing raw `invoke()` usage with narrow wrapper functions.

Dependency Inversion:
- Frontend improved by moving components onto a typed API layer.
- Rust still has direct orchestration over persistence/runtime details in `lib.rs`; that remains the main architectural debt.

## Summary

Main pre-refactor problems:
- One React god-file for control flow.
- One Rust god-file for bootstrap plus pure business logic.
- Scattered frontend IPC calls.
- Crate-root helper coupling from `state.rs` and `storage.rs`.

Main improvements delivered:
- Typed frontend IPC layer.
- Dedicated React controller hook plus view-model helpers.
- Pure Rust transcript and overlay modules.
- Rust state/persistence modules no longer reach back into `lib.rs` for defaults or indicator updates.
