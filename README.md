# How to install

## Linux

Locate your OBS plugin folder, then run those commands in the project folder (e.g. `~/.config/obs-studio`):

```
mkdir -p <obs_plugin_folder>/obs_mpris/bin/64bit/
cargo build --release
cp -f target/release/libobs_mpris.so <obs_plugin_folder>/plugins/obs_mpris/bin/64bit/obs_mpris.so
```

# How to use

This plugin provides you with a video source (MPRIS) and a video filter (Mpris Text Filter).
Both of them work by modifying a target text source's text with a configurable template.

## MPRIS

Add a text source (prefer unicode sources like [Pango](https://github.com/kkartaltepe/obs-text-pango)), then a MPRIS source to the scene.
Target the text source in the MPRIS source properties, and choose the player to monitor. Once it's done, your text should always display the test you put in your MPRIS source template.

## Mpris Text Filter

Add a text source, then add a Mpris Text Filter to it. The only difference with the video source version is the target text is the one the filter is attached to.
