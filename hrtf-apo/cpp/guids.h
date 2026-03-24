#pragma once
#include <guiddef.h>

// We hijack the WMALFXGFXDSP CLSID — this is the CLSID that the Sound Blaster
// endpoint references. By replacing the DLL in System32, audiodg.exe loads our
// code when it tries to create the original APO.
//
// We accept ANY CLSID in DllGetClassObject so we handle all APO requests
// that were previously routed to WMALFXGFXDSP.dll.

// Our "identity" CLSID for registration purposes — not actually used for loading,
// since we're loaded via DLL replacement.
// {A1B2C3D4-E5F6-7890-ABCD-EF0123456789}
DEFINE_GUID(CLSID_HrtfApo,
    0xa1b2c3d4, 0xe5f6, 0x7890, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89);
