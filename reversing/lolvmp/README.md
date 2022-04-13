# lolvmp

This is a tiny little hook DLL I wrote that made it easier for me to dump VMProtect 3.x executables when I didn't have a working anti-anti-debug solution for it.

It's meant to be used with an executable with "-noaslr" and a modified header that disables ASLR. The DLL restores the header back to what it should be after loading and redirects the tamper protection check to the original executable, bypassing it. It also suspends the process in the middle of security cookie initialization, which makes it much easier to dump.