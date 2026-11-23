# Background / multitasking upload

The Android client now starts user-triggered uploads in `UploadForegroundService` instead of tying the native Rust transfer to `MainActivity`.

## Behavior

- The upload continues when the user presses Home, opens another app, or Android recreates the Activity.
- A persistent system notification shows file name, percentage, confirmed transfer rate, and ETA.
- The service is declared as foreground service type `dataSync`.
- The latest transfer state is mirrored into `UploadServiceState` and persisted in SharedPreferences, so reopening the app reconnects the UI to the active transfer.
- `android:stopWithTask="false"` prevents removing the app task from automatically stopping the foreground service.
- The service returns `START_REDELIVER_INTENT`. If Android kills the process for memory pressure and later restarts the service, the original transfer Intent is redelivered and the Rust/server resume mechanism can continue from a safe offset.
- Files selected through Storage Access Framework are copied to `filesDir/pending_uploads/` instead of `cacheDir`, so Android cache cleanup cannot remove the source during a long transfer. The private copy is deleted after a successful upload.

## Manifest permissions

```xml
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />
```

Service declaration:

```xml
<service
    android:name=".UploadForegroundService"
    android:exported="false"
    android:stopWithTask="false"
    android:foregroundServiceType="dataSync" />
```

## Main classes

- `UploadForegroundService.kt` owns the long-running upload and foreground notification.
- `UploadServiceState.kt` stores/replays transfer status for Compose.
- `RustUploaderLib.kt` provides ACK-based progress from Rust/JNA.
- `MainActivity.kt` starts the service while the app is visible and renders the persisted service state.

## Important Android behavior

A foreground service substantially raises process priority, but Android can still terminate the process in extreme memory/system-health conditions. The app therefore relies on server-side resumable transfer plus `START_REDELIVER_INTENT` rather than assuming the process can never die.

A user force-stop, the Android task-manager Stop action, or some aggressive OEM battery-management actions can still terminate the app. No normal Android app can guarantee continuation after a user explicitly stops it.

For apps targeting Android 15 (API 35) or newer, `dataSync` foreground services have platform time limits while the app is in the background. Very long multi-hour transfers should eventually be migrated to Android user-initiated data-transfer jobs on supported API levels.

## Test checklist

1. Start the server and choose a file large enough to upload for several minutes.
2. Tap **GỬI FILE**.
3. Confirm the persistent notification shows percentage, speed and ETA.
4. Press Home and use another app for 1-2 minutes.
5. Reopen UDP File Transfer and confirm the UI reconnects to the same progress.
6. Swipe the app task away from Recents and verify the transfer continues in the notification.
7. Turn the screen off for a while, then verify progress continues/reconnects.
8. After completion, verify the notification changes to success and the item appears in History.
