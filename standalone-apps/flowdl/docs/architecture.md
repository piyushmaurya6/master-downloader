# FlowDL Architecture Guide

FlowDL is architected as a modular, 5-layer Windows desktop application using **Tauri 2**, **Rust**, **React 18**, and **TypeScript**.

---

## 1. Layered Architecture Overview

```mermaid
flowchart TD
    subgraph Presentation["1. Presentation Layer (React + TypeScript)"]
        UI["Pages & Components (HomePage, DownloadsPage, HistoryPage, SettingsPage)"]
        Store["AppContext Global Store & Event Listeners"]
        Bridge["Typed IPC Client (tauriBridge.ts)"]
    end

    subgraph IPC["Tauri 2 Constrained IPC Boundary"]
        Cmd["src-tauri/src/commands/mod.rs"]
    end

    subgraph AppService["2. Application Service Layer (Rust)"]
        Queue["QueueManager (queue/queue_manager.rs)"]
        ErrHandler["Error Classification & Recovery (services/error_handler.rs)"]
        AppUpdater["System A: App Updater (services/app_updater.rs)"]
        Diag["Sanitized Diagnostic Generator (services/diagnostic_service.rs)"]
    end

    subgraph EngineLayer["3. Download Engine Layer (Rust)"]
        Trait["DownloadEngine Trait & YtDlpEngine (downloader/engine.rs)"]
        Builder["Safe Argument Builder (downloader/download_manager.rs)"]
        Parser["Machine-Readable Progress Parser (downloader/progress_parser.rs)"]
        EngMgr["System B: EngineManager (dependencies/engine_manager.rs)"]
    end

    subgraph Persistence["4. Persistence Layer (Rust + SQLite)"]
        Hist["HistoryStore + Transactional Migrations (storage/history_store.rs)"]
        Settings["SettingsStore Atomic JSON (storage/settings_store.rs)"]
    end

    subgraph Platform["5. Platform Integration Layer (Rust + Win32)"]
        Win32["Native Win32 Dialogs & Explorer (platform/mod.rs)"]
        Proc["CREATE_NO_WINDOW Process Spawner (utils/mod.rs)"]
    end

    UI --> Store --> Bridge --> Cmd
    Cmd --> Queue & AppUpdater & Diag & EngMgr & Hist & Settings & Win32
    Queue --> Trait & Builder & Parser & ErrHandler & Hist & Proc
    EngMgr --> Proc
```

---

## 2. Layer Responsibilities

1. **Presentation Layer (`src/`)**:
   - Owns UI components, keyboard shortcuts, theme switching, and progress visualization.
   - Never constructs or executes raw shell commands.
2. **Application Service Layer (`src-tauri/src/queue/`, `src-tauri/src/services/`)**:
   - Owns URL validation, concurrency scheduling (`max_concurrent_downloads`), cancellation via `tokio::sync::watch`, automatic transient retries, error classification, and System A application updates.
3. **Download Engine Layer (`src-tauri/src/downloader/`, `src-tauri/src/dependencies/`)**:
   - Owns the `DownloadEngine` trait (`YtDlpEngine`), argument construction (`--progress-template`, `--print after_move:FLOWDL_FILEPATH|%(filepath)s`, `--js-runtimes deno:<path>`), line-by-line progress parsing, and `EngineManager` (System B).
4. **Persistence Layer (`src-tauri/src/storage/`, `src-tauri/migrations/`)**:
   - Owns `%LOCALAPPDATA%\FlowDL\flowdl.db` (SQLite with versioned transactional migrations in `schema_migrations`) and `%LOCALAPPDATA%\FlowDL\settings.json`.
5. **Platform Integration Layer (`src-tauri/src/platform/`, `src-tauri/src/utils/`)**:
   - Owns `CREATE_NO_WINDOW` (`0x08000000`) process spawning, native Win32 `SHBrowseForFolderW` folder selection, Windows Explorer reveal (`/select,<path>`), and Windows filename/path sanitization.

---

## 3. Typed Event Contracts

FlowDL emits structured events from the Rust backend to the React frontend:

| Event Name | Payload Type | Trigger |
| :--- | :--- | :--- |
| `download:queued` | `DownloadEventPayload` | Item validated and added to queue |
| `download:started` | `DownloadEventPayload` | Child process spawned (`downloading` / `retrying`) |
| `download:progress` | `DownloadEventPayload` | Numeric progress (`downloadedBytes`, `totalBytes`, `speedBytesPerSec`, `etaSeconds`) |
| `download:postprocessing` | `DownloadEventPayload` | FFmpeg stream merging, audio conversion, or metadata embedding |
| `download:completed` | `DownloadEventPayload` | Post-processing finished and final output file verified on disk |
| `download:failed` | `DownloadEventPayload` | Error classified with canonical `errorCode` and `recoveryAction` |
| `download:cancelled` | `DownloadEventPayload` | User stopped or cancelled the download |
| `engine:update-available` | `EngineUpdateInfo` | Newer `yt-dlp` release detected on official GitHub releases |
| `app:update-available` | `AppUpdateInfo` | Newer FlowDL application release detected |
| `queue-updated` | `Vec<DownloadItem>` | Full queue snapshot synchronized after state transitions |

---

## 4. Download State Machine

```mermaid
stateDiagram-v2
    [*] --> queued: enqueue_download()
    queued --> analyzing: Scheduler slot available
    analyzing --> downloading: Streams resolved
    downloading --> postprocessing: Media streams downloaded
    postprocessing --> completed: FFmpeg merge/convert exit 0
    downloading --> retrying: Transient NETWORK_ERROR (attempt <= max_retries)
    retrying --> downloading: Backoff timer elapsed
    downloading --> stopped: User clicks Stop (keeps .part file)
    stopped --> queued: User clicks Resume (--continue)
    downloading --> cancelled: User confirms Cancel
    downloading --> failed: Non-retryable error or retries exhausted
    failed --> queued: User clicks Retry or Update Engine & Retry
```
