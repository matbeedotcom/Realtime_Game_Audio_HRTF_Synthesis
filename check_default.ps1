Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
class MMDeviceEnumerator {}

[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator {
    int EnumAudioEndpoints(int dataFlow, int dwStateMask, out IntPtr ppDevices);
    int GetDefaultAudioEndpoint(int dataFlow, int role, [MarshalAs(UnmanagedType.Interface)] out object ppEndpoint);
}

[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDevice {
    int Activate(ref Guid iid, int dwClsCtx, IntPtr pActivationParams, [MarshalAs(UnmanagedType.IUnknown)] out object ppInterface);
    int OpenPropertyStore(int stgmAccess, [MarshalAs(UnmanagedType.IUnknown)] out object ppProperties);
    int GetId([MarshalAs(UnmanagedType.LPWStr)] out string ppstrId);
    int GetState(out int pdwState);
}
"@

$enum = New-Object MMDeviceEnumerator
$iEnum = [IMMDeviceEnumerator]$enum

# eRender=0, eConsole=0
$device = $null
$hr = $iEnum.GetDefaultAudioEndpoint(0, 0, [ref]$device)
$iDev = [IMMDevice]$device
$id = $null
$iDev.GetId([ref]$id)
Write-Host "Default render endpoint (Console): $id"

# eRender=0, eMultimedia=1
$device2 = $null
$hr = $iEnum.GetDefaultAudioEndpoint(0, 1, [ref]$device2)
$iDev2 = [IMMDevice]$device2
$id2 = $null
$iDev2.GetId([ref]$id2)
Write-Host "Default render endpoint (Multimedia): $id2"
