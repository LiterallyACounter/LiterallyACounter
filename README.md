# Literally A Counter

:gb: **English** · :fr: [Français](README.fr.md)

A tiny counter for gamers and streamers. Counters live in small borderless windows, on screen or as an OBS overlay, and you count with keyboard or mouse shortcuts **while the game keeps the focus**.

Example: count the chests you open in a match, plus a running total across matches and days.

## Features

- **Two counters**: a **total** (saved, restored at startup) and a **session** counter (starts at 0 every launch). Show or hide each one.
- **Global shortcuts**, keyboard or mouse (buttons and wheel notches), single keys or combinations. **Keys and clicks are never blocked**: they reach the game normally, with no added delay.
- +1, −1, +5, −5, reset, undo the last reset. The total's reset needs a second key to confirm, with no dialog box stealing the focus.
- **Free display format** with variables: `{value}`, `{goal}`, `{remaining}`, `{ratio}`, `{ratio:p}`. Text in brackets only shows when there is a goal: `{value}[ / {goal}]` gives `3 / 10`, or `3` without a goal.
- Installed fonts, colors (goal reached / not reached), transparent background, rounded corners, always on top.
- **A sound for each action**: Windows sounds, built-in sounds, beeps or your own WAV / MP3 files. Volume, mute, audio output choice.
- English and French interface, more languages through language packs.
- A single ~5.5 MB `.exe`, no installation, negligible CPU use.

Windows only for now. Linux and macOS are planned.

## Download

Get `literally_a_counter.exe` from the [Releases](../../releases) page and run it. Settings are stored in `%APPDATA%\LiterallyACounter\`.

Prefer not to run a prebuilt exe? [Build it yourself](#build-from-source): it takes a few minutes.

## Default shortcuts

| Action | Shortcut | Counters |
|---|---|---|
| +1 | Num + · Ctrl + middle click · mouse forward button | session **and** total |
| −1 | Num − · mouse back button | session **and** total |
| +5 / −5 | Shift + Num + / Shift + Num − | session **and** total |
| Reset session | Num 0 | session |
| Reset total | Ctrl + Num 0, then Num Enter within 3 s | total |
| Undo the last reset | Ctrl + Num . | the last one reset |

Left / right click on a counter window: +1 / −1 on that counter. Hover a counter and click **•••** to open its menu (set the value, settings…).

Everything can be changed in the settings window: press the new shortcut to record it.

## Anti-cheat

> [!WARNING]
> The app watches the keyboard the same way OBS, Discord or LiveSplit do: a **passive** hook that only reads key presses, without blocking or sending any. It never reads or touches the game itself.
>
> That is the safest known method, but no anti-cheat gives guarantees: use it at your own risk with the strictest ones (such as **Vanguard**).
>
> **A warning, a kick or a ban with an anti-cheat?** Please [open an issue](../../issues) with the game and the anti-cheat name, so it can be fixed, or at least listed here for everyone.

<a name="streaming-with-obs"></a>
<details>
<summary><h2>Streaming with OBS</h2></summary>

Add a **Window Capture** source, select the counter window, and set the capture method to **Windows Graphics Capture** (WGC; its label depends on the OBS version and language). With the other methods, the transparent background turns black.

</details>

<a name="language-packs"></a>
<details>
<summary><h2>Language packs</h2></summary>

English and French are built in. To add a language, copy `lang/en.ftl`, translate the texts, and save it as `lang/xx.ftl` next to the exe (`xx` = language code), with the language name in `language-name`. It shows up in the language list at the next launch. Missing texts fall back to English.

</details>

<a name="build-from-source"></a>
<details>
<summary><h2>Build from source</h2></summary>

You don't have to trust a prebuilt exe: the app builds from this source code with the official Rust tools.

1. **Install Rust** from [rustup.rs](https://rustup.rs) (`rustup-init.exe`). When it asks, let it install the **Visual Studio C++ build tools**: Rust needs them on Windows.
2. **Get the source code**: with `git clone` and the address given by the green **Code** button, or download the ZIP from that same button and extract it.
3. **Build**, in a terminal opened in the project folder:

   ```
   cargo build --release
   ```

   The first build downloads the dependencies and takes a few minutes.
4. The app is `target\release\literally_a_counter.exe`.

To run the automatic tests: `cargo test`.

The icon is already generated (`assets/icon/`). Only to change it: `python tools/gen_icon.py assets/icon preview.png` (needs Python and Pillow).

</details>

## Credits

Built-in sounds: "Interface Sounds" by [Kenney](https://www.kenney.nl) (CC0). Details in [CREDITS.md](CREDITS.md).

Release notes: [CHANGELOG.md](CHANGELOG.md).

## License

[GPL-3.0-or-later](LICENSE): you may use, study, modify and share the app; modified versions you distribute must stay under the same license, with their source code.
