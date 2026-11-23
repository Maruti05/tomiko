# Tomiko — Master Product Requirement Document

> **Implementation note (November 2026):** The technical stack has been revised
> to Kotlin + Jetpack Compose + Rust + Bevy 0.20.1 + GameActivity. Where this PRD
> conflicts with `DEPENDENCIES.md`, `docs/ARCHITECTURE.md`, or
> `docs/CHARACTER_PIPELINE.md`, the newer documents are authoritative.

## 1. Product Vision

Tomiko is an **offline-first, privacy-centric** Android application for children aged 3–8.  
The core interaction loop: a child speaks to a friendly 3D on-screen character → the character repeats the phrase in a funny voice → the character performs automatic lip-sync + context-aware animations.

### Core Strategic Constraints

| Constraint | Rationale |
|---|---|
| **Zero internet dependencies** | No cloud calls, no CDN asset loading, no analytics pings. APK is fully self-contained. |
| **No authentication or login** | No accounts, no user profiling, no sign-up screens. |
| **No advertisement SDKs** | COPPA / GDPR-K compliance by design; clean UX for children. |
| **Zero structured databases** | No Room, SQLite, or Realm. All state is file-system based (JSON + binary blobs). |
| **Entirely offline** | No crash reporting SDK, no remote config, no push notifications. |

---

## 2. Target Devices

| Category | Details |
|---|---|
| **Minimum Android version** | Android 11 (API 30) |
| **Target Android version** | Android 15–16 (API 35–36) |
| **RAM floor** | 2 GB (API 30 devices) |
| **Screen sizes** | 4.7″ phones → 14″ foldables |
| **Form factors** | Phones, tablets, foldables, chromebooks (non-keyboard) |
| **Architecture** | arm64-v8a (primary), armeabi-v7a (fallback) |
| **Orientation** | Locked to **landscape** on tablets; **portrait** or **sensor** on phones (configurable at build). |

---

## 3. Core User Flow

```
[Splash Screen] → [Home / Character Select] → [Talking Screen] → [Playback Screen]
                                                       ↑                    |
                                                       └──── (repeat) ──────┘
```

1. **Splash** — minimal loading of bundled 3D assets.
2. **Home** — choose from 1–3 pre-loaded characters (bundled in APK).
3. **Talking** — 3D character on screen, big "Tap to Talk" button. Child speaks.
4. **Playback** — character repeats recorded audio with voice effect + lip-sync animation. Child can replay or go back.

---

## 4. Key Features

| Feature | Description |
|---|---|
| **3D Character Rendering** | Google Filament via SceneView; glTF 2.0 + KTX2 textures |
| **Voice Recording** | Ultra-low-latency AudioRecord, max 10 s clips |
| **Voice Effects** | Pitch-shift, formant-filter, robot/reverb presets (native DSP) |
| **Lip-Sync** | Energy-based viseme detection → morph-target blendshapes |
| **Animation System** | Idle → Listening → Talking → Happy state machine |
| **Adaptive UI** | Edge-to-edge, foldable posture detection, Material 3 |
| **Soundboard** | Pre-recorded funny phrases for passive play |
| **Parental Gate** | Simple math-challenge before any settings access |

---

## 5. Non-Functional Requirements

| Requirement | Target |
|---|---|
| **Frame rate** | 45–60 FPS on 2 GB devices; stable 60 FPS on 4 GB+ |
| **App size** | ≤ 60 MB (bundled with 1 low-poly character) |
| **Record latency** | < 300 ms from button press to recording start |
| **Playback latency** | < 200 ms from tap to audio + animation start |
| **Battery drain** | < 5% per 10 min of continuous play |
| **Memory** | Peak heap ≤ 128 MB; native (GPU) ≤ 64 MB |
| **Thermal** | No sustained throttling after 15 min play session |

---

## 6. App Identity

| Field | Value |
|---|---|
| **App name** | Tomiko |
| **Package name** | `com.vedica.labs.ind.app.talkin.tomiko` |
| **Min SDK** | 30 |
| **Target SDK** | 36 |
| **Build system** | Gradle Kotlin DSL (`.gradle.kts`) |
| **Versioning** | Semantic: `major.minor.patch` (e.g., `1.2.3`) |

---

## 7. Document Map

| # | Document | Focus |
|---|---|---|
| 01 | [TECHNICAL-STACK](./01-TECHNICAL-STACK.md) | Language, frameworks, libraries, versions |
| 02 | [ARCHITECTURE](./02-ARCHITECTURE.md) | Clean Architecture + MVVM layers, package tree |
| 03 | [3D-CHARACTER](./03-3D-CHARACTER.md) | SceneView, glTF pipeline, character spec |
| 04 | [AUDIO-PIPELINE](./04-AUDIO-PIPELINE.md) | Record, playback, voice effects, lip-sync DSP |
| 05 | [ANIMATION](./05-ANIMATION.md) | State machine, triggers, blend transitions |
| 06 | [UI-DESIGN](./06-UI-DESIGN.md) | Material 3, edge-to-edge, foldable/tablet |
| 07 | [PERFORMANCE](./07-PERFORMANCE.md) | GPU, memory, battery, APK size budgets |
| 08 | [COMPATIBILITY](./08-COMPATIBILITY.md) | Android 11–16, screen sizes, accessibility |
| 09 | [TESTING](./09-TESTING.md) | Unit, integration, manual QA checklists |
| 10 | [RELEASE](./10-RELEASE.md) | Play Store checklist, signing, app bundle |
| 11 | [LLM-PROMPTS](./11-LLM-PROMPTS.md) | Module-by-module generation instructions |
| 12 | [CODING-STANDARDS](./12-CODING-STANDARDS.md) | Naming, lint, error handling, conventions |
