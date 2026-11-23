# Android UI Redesign

The Android client UI was redesigned to make the normal upload flow simpler and safer.

## Main UX changes

- One primary action: **Gửi file**.
- Direct upload vs WorkManager is now a switch instead of two competing buttons.
- File selection is shown as a compact file card with filename and size.
- Large file copying from Android's Storage Access Framework runs on `Dispatchers.IO` instead of blocking the UI thread.
- Server IP and optional password stay in the main flow.
- UDP port, HTTP port and block size moved to **Cài đặt nâng cao**.
- Added input validation before starting an upload.
- Added a clear status card with idle/running/success/error states and an indeterminate progress bar while the native uploader is busy.
- Upload history moved to its own bottom-navigation destination.
- History cards are shorter and make the result, size, time and Hash ID easier to scan.
- Added one-tap Hash ID copy.
- The demo-file generator is still available but collapsed by default.
- WorkManager jobs require a connected network before starting.
- Removed the obsolete `READ_EXTERNAL_STORAGE` permission because file selection uses `OpenDocument`/SAF.
- The UI follows the system light/dark theme.

## Files changed

- `android-app/app/src/main/java/com/filetransfersystem/MainActivity.kt`
- `android-app/app/build.gradle`
- `android-app/app/src/main/AndroidManifest.xml`

## Build note

The source was statically reviewed in the editing environment. A full Gradle build could not be completed because the Gradle 8.5 distribution is not cached locally and outbound DNS/network access is unavailable.

On a development machine, verify with:

```bash
cd android-app
./gradlew :app:assembleDebug
```

For the full project build, use the existing root build scripts after the Android native `.so` libraries are generated.
