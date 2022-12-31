# portalrtxvbfix
This is a dead-simple patch to fix the `failed to lock vertex buffer in CMeshDX8::LockVertexBuffer` crash in Portal RTX when loading certain Half-Life 2 maps.

Just apply the .bps file to `steamapps\common\PortalRTX\bin\shaderapidx9.dll` using a tool such as [Floating IPS](https://www.romhacking.net/utilities/1040/).

## Technical details
For some reason, the game tries to allocate a vertex buffer with a zero-sized vertex format in certain HL2 maps. The vertex buffer itself is created just fine, but it crashes once the game tries to lock it later. This patch simply patches `CVertexBufferBase::VertexFormatSize` to return 4 when it tries to return 0.