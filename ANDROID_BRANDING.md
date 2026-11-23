# Android app branding

The Android launcher icon uses the modern file-transfer mark and now supports Android adaptive and themed icons.

## Behavior by Android version

- Android 7.x and older supported by this app (`minSdk 24`): density-specific PNG launcher icon fallback.
- Android 8-12 (API 26-32): Adaptive Icon with separate background and foreground layers so launchers can crop it to circle, squircle, rounded square, and other device shapes.
- Android 13+ (API 33+): the same Adaptive Icon plus a monochrome layer for Material You / Themed Icons.

## Branding assets

- `android-app/branding/app_icon_1254.png` - high-resolution source export.
- `android-app/branding/app_icon_preview.png` - 512x512 preview.
- `android-app/branding/play_store_icon_512.png` - 512x512 Play Store-ready export.

## Android launcher resources

- `android-app/app/src/main/res/mipmap-*/ic_launcher.png` - legacy launcher fallback, mdpi through xxxhdpi.
- `android-app/app/src/main/res/mipmap-*/ic_launcher_round.png` - legacy round-icon fallback.
- `android-app/app/src/main/res/drawable/ic_launcher_foreground.xml` - adaptive foreground layer.
- `android-app/app/src/main/res/drawable/ic_launcher_monochrome.xml` - one-color themed-icon mark.
- `android-app/app/src/main/res/values/colors.xml` - adaptive icon background color.
- `android-app/app/src/main/res/mipmap-anydpi-v26/ic_launcher*.xml` - adaptive icons for API 26-32.
- `android-app/app/src/main/res/mipmap-anydpi-v33/ic_launcher*.xml` - adaptive + monochrome icons for API 33+.

The manifest references `@mipmap/ic_launcher` and `@mipmap/ic_launcher_round`; Android automatically selects the best version for the device API level and launcher.
