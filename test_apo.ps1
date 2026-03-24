# Test if our APO COM object can be instantiated and initialized
# This mimics what the audio engine does

$clsid = "{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}"
Write-Host "Testing APO COM object: $clsid"

try {
    $type = [Type]::GetTypeFromCLSID([Guid]$clsid)
    $obj = [Activator]::CreateInstance($type)
    Write-Host "SUCCESS: COM object created"
    Write-Host "Type: $($obj.GetType())"

    # Try to query the interfaces
    Write-Host "Object: $obj"
} catch {
    Write-Host "FAILED: $($_.Exception.Message)"
}

# Also check if the DLL can be loaded directly
Write-Host ""
Write-Host "DLL load test:"
try {
    $dll = [System.Reflection.Assembly]::LoadFile("C:\ProgramData\HrtfApo\hrtf_apo.dll")
    Write-Host "Loaded as .NET assembly (unexpected for native DLL)"
} catch {
    Write-Host "Not a .NET assembly (expected for native COM DLL)"
}

# Check DLL with LoadLibrary
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class NativeLoader {
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr LoadLibrary(string lpFileName);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool FreeLibrary(IntPtr hModule);

    [DllImport("kernel32.dll")]
    public static extern uint GetLastError();
}
"@

$handle = [NativeLoader]::LoadLibrary("C:\ProgramData\HrtfApo\hrtf_apo.dll")
if ($handle -ne [IntPtr]::Zero) {
    Write-Host "DLL LoadLibrary: SUCCESS (handle=$handle)"
    [NativeLoader]::FreeLibrary($handle) | Out-Null
} else {
    $err = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
    Write-Host "DLL LoadLibrary: FAILED (error=$err)"
}
