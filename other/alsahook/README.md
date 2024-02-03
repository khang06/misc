# alsahook
A hook library to get the low audio latency of Wine + PipeWire osu!stable, but on native Linux osu!lazer.

**DISCLAIMER: I have no idea if the anticheat cares about this. Use at your own risk!**

# Usage
1. Build the library with `make` (requires patchelf)
2. Run osu!lazer with `LD_LIBRARY_PATH` pointing to this directory (e.g. `LD_LIBRARY_PATH=/home/khangaroo/github/misc/other/alsahook ./osu.AppImage`)
