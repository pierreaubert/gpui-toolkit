# Store submission checklist (GPUI Python mobile apps)

Machinery shipped in Phase 5; credentials and store accounts are the
app owner's. Nothing below uploads anything by itself.

## Shared

- Set the app version once in `gpui-mobile.toml` (`[app] version`); the
  generated Just recipes feed it to both stores (`MARKETING_VERSION` on
  iOS via xcodebuild settings from the project, `gpuiVersionName` on
  Android). Bump `CURRENT_PROJECT_VERSION` / `GPUI_VERSION_CODE` per upload.
- Replace the default icon by dropping `icon-1024.png` next to
  `main_app.py` in `app/` before building; the stage scripts prefer it
  over the toolkit default.
- Audit your Python code's required-reason API and data-collection use
  before submitting on either store; the shipped privacy material is a
  starting point that covers the host shell only.

## iOS

- `just ios` builds with manual signing: set `IOS_DEVELOPMENT_TEAM` and
  `IOS_SIGN_IDENTITY` (an "Apple Distribution" identity for store builds).
- `PrivacyInfo.xcprivacy` ships at the bundle root declaring no tracking
  and the shell's own required-reason APIs; CPython's C extensions carry
  per-framework privacy files installed by the build phase. Re-audit
  against your app's actual API use and declare collected data in App
  Store Connect (privacy nutrition labels).
- App icon comes from `Assets.xcassets/AppIcon.appiconset` (single
  1024px), launch screen is the empty `UILaunchScreen` (black).
- Archive via Xcode Organizer or `xcodebuild archive` on the generated
  project; validate in Transporter before submission.

## Android

- New apps must target API 36 (enforced by Play since 2026-08-31); the
  Gradle hosts here already set `compileSdk`/`targetSdk` 36, `minSdk` 26.
- Sign release builds by exporting (never committing):
  `GPUI_ANDROID_KEYSTORE`, `GPUI_ANDROID_KEYSTORE_PASSWORD`,
  `GPUI_ANDROID_KEY_ALIAS`, `GPUI_ANDROID_KEY_PASSWORD`, then
  `just android-aab`. Without them the release AAB stays unsigned, which
  is useful for CI checks only. Prefer Play App Signing: keep the upload
  key in your password manager / CI secrets.
- Fill the Play data-safety form to match what your Python code collects;
  the host itself collects nothing.

## Sizes (measured 2026-11-14, tutorial app)

Trimming removes the stdlib `test` tree, `idlelib`, `turtledemo`,
`ensurepip` wheels, and `__pycache__` at bundle time on both platforms.

| Artifact | Before | After |
| --- | --- | --- |
| iOS `.app` (simulator) | 333M | 107M |
| iOS `python/` home | 237M | 14M |
| Android staged stdlib assets | 66M | 23M |
| Android release `.aab` | 42M | 42M |

First launch after trimming is slower (CPython compiles imports without
`__pycache__`); the iOS host may briefly show its queue-busy banner on
cold start. If that matters for your app, drop the `__pycache__` line
from the trim step in `project.yml` / `stage-android-resources.sh`.
