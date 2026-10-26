<div align="center">

<h1>slot. XMB</h1>

<p><b>A PSP-style XrossMediaBar skin for <a href="https://github.com/BrandonKowalski/slot">slot</a>, the Game Boy-centric frontend for the Anbernic RG SP.</b></p>

<p>slot is made by <a href="https://github.com/BrandonKowalski">Brandon Kowalski</a>. All of the emulation, saves, link play and everything else under the skin is his work.</p>

<img src=".github/resources/xmb/startup.gif" width="72%" alt="Powering on, then scrolling the games, jumping a letter with R1 and back with L1, and moving between categories">

</div>

---

> [!NOTE]
> This is an unofficial fork. It isn't affiliated with Sony or with the upstream slot project. Bugs in the XMB skin belong here, not upstream.

# What is this?

[slot](https://github.com/BrandonKowalski/slot) by Brandon Kowalski is a frontend for the Anbernic RG SP that plays GBA, GBC and GB games. This fork keeps slot's emulation, save handling, link play and settings, and replaces its menus with a PSP-style bar.

Everything here is built behind a `xmb` cargo feature. Without that feature, slot builds and behaves exactly like upstream.

# What does it look like?

<p align="center">
<img src=".github/resources/xmb/01_home_gba.png" width="48%" alt="The Game Boy Advance column with game carts, in Violet">
<img src=".github/resources/xmb/12_gb_labels.png" width="48%" alt="The Game Boy column on a pale pink colour">
</p>
<p align="center">
<img src=".github/resources/xmb/07_classic_labels.png" width="48%" alt="Labels instead of cartridges, on a magenta colour">
<img src=".github/resources/xmb/21_about.png" width="48%" alt="The About page, in Cyan">
</p>
<p align="center">
<img src=".github/resources/xmb/03_settings.png" width="48%" alt="The Settings column, in Green with Classic waves">
<img src=".github/resources/xmb/05_colour_picker.png" width="48%" alt="Picking a colour in Theme Settings, previewing Red">
</p>
<p align="center">
<img src=".github/resources/xmb/13_volume.png" width="48%" alt="The PSP-style volume gauge, in Indigo with Classic waves">
<img src=".github/resources/xmb/14_in_game.png" width="48%" alt="The in-game menu, dark while the game is paused">
</p>
<p align="center">
<img src=".github/resources/xmb/41_modern_cyan.png" width="48%" alt="Modern, the PS3's wave and sparkles, in Cyan">
<img src=".github/resources/xmb/45_modern_night.png" width="48%" alt="Modern by the month, at night">
</p>
<p align="center">
<img src=".github/resources/xmb/17_clock_settings.png" width="48%" alt="Date and Time Settings, in Gold with Classic waves">
<img src=".github/resources/xmb/boot.png" width="48%" alt="The boot picture">
</p>

# What's different

**The bar**

- Categories run across the screen and items run down it, with the PSP's sizes, spacing, highlight glow and the small arrow beside an open item.
- Game names show only on the game under the cursor.
- L1 and R1 jump back or on a letter in the game list, gliding there smoothly.
- The clock and battery sit at the top right and hide whenever a side menu is open. A battery percentage can be shown next to the battery icon.

**Waves and colours**

- **Original** and **Classic** wave backgrounds, both drawn on the GPU.
- **Modern**: the PS3's wave and its sparkles, ported from [faquino's reverse engineering](https://github.com/faquino/PlayStation-3-XMB) of the console's own code. It follows the time of day from dawn to night, and the sparkles drift as you move along the bar. On By Month it uses the PS3's own colour for each month, and any other colour gives it the bar's colour as its backdrop.
- 34 background colours fitted from screenshots of a real PSP, each with its own edge glow and shading. There's also a By Month option that changes the colour each month.
- A **Wallpaper** background replaces the waves with your own picture.

**Start-up and switch-off**

- On power-up the screen shows a boot picture that reads "slot. XMB".
- The start-up animation takes over from that picture without a jump. The lines brighten, colour spreads out from them, and the waves grow out of them.
- Games and settings load while the animation plays. If loading takes longer, the waves stay up with a pulsing ring at the lower left.
- With Modern, the PS3's own start-up plays instead: from black, its wave rises through the boot picture's lines and the colour comes up behind it.
- Switching off plays the animation in reverse.
- Booting straight back into a game you left running skips the animation and goes from the boot picture to the game.

**Settings**

- Theme Settings: Background (Original, Classic, Modern or Wallpaper), Color (or Wallpaper), Game Icons, and Dark Pause Menu.
- Date & Time Settings: a PSP-style clock editor, date format, 12- or 24-hour time, and time zone.
- Volume, brightness and screen-temperature gauges in the PSP's style.
- About sits at the bottom of Settings and scrolls.
- While a game is paused, the in-game menu goes dark whatever the colour, in either wave style, with the game showing only faintly behind it, inspired by the PS3. Turn off **Dark Pause Menu** in Theme Settings to keep your colour there instead.

**Power use**

- When nothing is moving on the home screen, the bar draws at 20 frames per second instead of 60, so the device runs cooler.

# Install

Download the latest release from this fork's [releases page](../../releases/latest). It's the complete slot card layout, with the same cores, scripts and folders, plus the XMB build of `System/slot`.

- **New install:** follow the [slot guide](https://slot-cfw.fyi), using this fork's release zip in place of slot's own.
- **Updating, or switching from slot v1.9.0:** copy `System/slot` from the zip over the one on your SD card. Your games, saves and settings stay as they are.

The boot picture is written to the device the first time the new build runs. It shows from the following boot onwards.

## Your PSP's own background colours (optional)

The colours are fitted from screenshots of a PSP. If you have a PSP or PS Vita with custom firmware, you can give both wave styles your console's exact backgrounds. Copy these files from `flash0:/vsh/resource/` on your console to `System/xmb/` on the SD card:

- `13-27.bmp`
- `01-12_03g.bmp`, or `01-12.bmp` on a PSP-1000 or PSP-2000

They aren't included here, and nothing changes if they're missing.

## Sounds (optional)

The bar plays short sound effects if it finds them on the SD card. **They aren't included in this repo**, so you need to supply your own. Each must be a 16-bit PCM WAV, at any sample rate, mono or stereo.

| File | Plays when |
| --- | --- |
| `System/sounds/opening.wav` | the start-up animation runs |
| `System/sounds/cursor.wav` | you move the selection |
| `System/sounds/confirm.wav` | you open or choose something |
| `System/sounds/cancel.wav` | you back out |
| `System/sounds/start.wav` | the device boots straight back into a game |

If a file is missing, that sound is simply silent.

# Build

These are the same steps as upstream, with the `xmb` feature on:

```sh
cargo build --profile device --target aarch64-unknown-linux-gnu \
  -p slot --no-default-features --features device,xmb
```

The XMB screenshots in this README come from a software-rendered test:

```sh
XMB_SHOTS=shots cargo test --release -p slot \
  --no-default-features --features device,xmb --test xmb_shots
```

# Credits

- **[slot](https://github.com/BrandonKowalski/slot) by [Brandon Kowalski](https://github.com/BrandonKowalski).** This fork is built on slot v1.9.0 and wouldn't exist without it. Read the [slot guide](https://slot-cfw.fyi) and the [upstream changelog](https://github.com/BrandonKowalski/slot/blob/main/CHANGELOG.md), and if you enjoy slot, you can [support Brandon on Ko-fi](https://ko-fi.com/kwallbmoc).
- [BaseOS](https://github.com/pvaibhav/BaseOS), the system slot runs on.
- **Modern** is ported from [PlayStation-3-XMB](https://github.com/faquino/PlayStation-3-XMB) by faquino, a fork of Mart's [linkev/PlayStation-3-XMB](https://github.com/linkev/PlayStation-3-XMB), and from [Nagare](https://github.com/bungholesurfers/Nagare), its Rust port. faquino worked out the wave, the sparkles and the backdrop from the console's firmware in RPCS3. Their code is MIT licensed; the notice is in [`crates/slot-ui/src/xmb/modern/LICENSE-MIT`](crates/slot-ui/src/xmb/modern/LICENSE-MIT).
- Fonts: [Saira](https://github.com/Omnibus-Type/Saira) and [Montserrat](https://github.com/JulietaUla/Montserrat), both under the SIL Open Font License.
- The look is modelled on Sony's PSP and PS3 XrossMediaBar. PlayStation, PSP and XrossMediaBar are trademarks of Sony. This fork contains no Sony code, artwork or audio. Modern's PS3 month colours and sparkle tints are numbers fitted to the firmware's textures, which faquino's project ships, not the textures themselves.

# License

GPL-3.0, the same as upstream. See [LICENSE](LICENSE). The Modern background's ported code keeps its MIT licence ([`LICENSE-MIT`](crates/slot-ui/src/xmb/modern/LICENSE-MIT)).

slot is the work of Brandon Kowalski. This fork modifies slot v1.9.0 (commit `20703ed`): the changes, made in October 2026, add the `xmb` feature and its art, fonts, animation and settings. The full history of both is in this repository's commits.
