# Android real-time upload progress

The Android client now receives real transfer progress directly from `client_lib` through a JNA callback.

## What is reported

- Confirmed bytes: counts only chunks whose ACK has been received from the server.
- Total file bytes.
- Effective confirmed bytes/second for the current transfer session.
- Android derives percentage and ETA from these values.

The Rust callback is throttled to roughly one update every 250 ms to avoid excessive JNI/JNA crossings and UI/WorkManager overhead.

## Direct upload flow

`client_lib` exports:

```text
rtk_upload_file_with_password_and_progress(..., progress_callback)
```

The callback receives:

```text
confirmed_bytes, total_bytes, bytes_per_second
```

`RustUploader.performUpload()` maps this into `UploadProgress`, and Compose renders a determinate progress bar with percentage, transferred size, transfer rate, and ETA.

## WorkManager flow

`UploadWorker` sends the native progress samples through a conflated Kotlin `Channel`. The collector publishes only the latest samples into WorkManager progress data using:

- `KEY_PROGRESS_BYTES`
- `KEY_PROGRESS_TOTAL`
- `KEY_PROGRESS_BPS`

The activity observes `WorkInfo.progress` and renders the same UI as direct mode.

## Build requirement

Because a new native symbol was added, rebuild `libclient_lib.so` before rebuilding the APK. From the repository root:

```bash
./build_android.sh debug
```

or for release:

```bash
./build_android.sh
```

Using an older `.so` with the new Kotlin code will return error `-98` because the progress-enabled symbol is missing.
