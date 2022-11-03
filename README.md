# How to install

Locate your OBS plugin folder, then run those commands in the project folder (e.g. `~/.config/obs-studio`):

```
mkdir -p <obs_plugin_folder>/obs_mpris/bin/64bit/
cargo build --release
cp -f target/release/libobs_mpris.so <obs_plugin_folder>/plugins/obs_mpris/bin/64bit/obs_mpris.so
```

# How to use

Create a Text source and a MPRIS source on your scene. Then setup the MPRIS source to setup its target Text source and the monitored player.

# TODO

- Give the list of MPRIS players instead of letting the user type anything in the player property field
- Use all metadata instead of only song title + allow user to give a text template
