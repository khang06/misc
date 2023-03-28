# bc7e
When importing things, Dota 2's Hammer sometimes complains about a missing "bc7e.dll". Fortunately, this DLL appears to just be a thin wrapper around an open-source project.

# Building
1. Compile the ISPC file in https://github.com/BinomialLLC/bc7e
2. Copy the generated $bc7e\* files to the bc7e folder (same folder as bc7e.vcxproj, dllmain.cpp, etc)
3. Build the solution as x64/Release
4. Copy bc7e.dll to `dota 2 beta/game/bin/win64`