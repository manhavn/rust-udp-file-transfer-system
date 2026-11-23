# Adaptive and themed launcher icon

The launcher branding is split into Android resource layers instead of relying only on a pre-rounded bitmap.

## Resource selection

| Device | Resource used |
| --- | --- |
| API 24-25 | `mipmap-*/ic_launcher.png` / `ic_launcher_round.png` |
| API 26-32 | `mipmap-anydpi-v26/ic_launcher*.xml` |
| API 33+ | `mipmap-anydpi-v33/ic_launcher*.xml` with `monochrome` |

## Notes

The foreground artwork is intentionally kept inside the adaptive-icon safe area so OEM launchers can apply their own mask without clipping the file-transfer arrows. The monochrome layer is a simplified one-color file + bidirectional-transfer mark, allowing Android 13+ to tint it using the user's Material You theme.

When changing the brand later, update both `ic_launcher_foreground.xml` and `ic_launcher_monochrome.xml`; do not replace only the legacy PNG files.
