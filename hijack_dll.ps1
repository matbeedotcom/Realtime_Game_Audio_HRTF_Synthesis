$log = "C:\ProgramData\HrtfApo\hijack.log"
$originalDll = "C:\WINDOWS\System32\WMALFXGFXDSP.dll"
$backupDll = "C:\ProgramData\HrtfApo\WMALFXGFXDSP.dll.bak"
$ourDll = "C:\ProgramData\HrtfApo\hrtf_apo.dll"

try {
    "=== DLL File Hijack $(Get-Date) ===" | Out-File $log -Encoding ascii

    # Stop audio
    net stop audiosrv /y 2>&1 | Out-File $log -Append -Encoding ascii
    net stop AudioEndpointBuilder /y 2>&1 | Out-File $log -Append -Encoding ascii
    Start-Sleep 2
    Stop-Process -Name audiodg -Force -ErrorAction SilentlyContinue
    Start-Sleep 1

    # Backup original
    if (-not (Test-Path $backupDll)) {
        Copy-Item $originalDll $backupDll -Force
        "Backed up original to $backupDll" | Out-File $log -Append -Encoding ascii
    } else {
        "Backup already exists" | Out-File $log -Append -Encoding ascii
    }

    # Take ownership and replace
    takeown /f $originalDll 2>&1 | Out-File $log -Append -Encoding ascii
    icacls $originalDll /grant Administrators:F 2>&1 | Out-File $log -Append -Encoding ascii
    Copy-Item $ourDll $originalDll -Force
    "Replaced $originalDll with our DLL" | Out-File $log -Append -Encoding ascii

    # Verify
    $origHash = (Get-FileHash $originalDll).Hash
    $ourHash = (Get-FileHash $ourDll).Hash
    "System32 hash: $origHash" | Out-File $log -Append -Encoding ascii
    "Our hash: $ourHash" | Out-File $log -Append -Encoding ascii
    "Match: $($origHash -eq $ourHash)" | Out-File $log -Append -Encoding ascii

    # Clear debug log
    Remove-Item "C:\ProgramData\HrtfApo\debug.log" -ErrorAction SilentlyContinue

    # Restart audio
    net start AudioEndpointBuilder 2>&1 | Out-File $log -Append -Encoding ascii
    net start audiosrv 2>&1 | Out-File $log -Append -Encoding ascii

    Start-Sleep 3
    if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
        "=== debug.log ===" | Out-File $log -Append -Encoding ascii
        Get-Content "C:\ProgramData\HrtfApo\debug.log" | Out-File $log -Append -Encoding ascii
    } else {
        "No debug.log yet" | Out-File $log -Append -Encoding ascii
    }

    "Done" | Out-File $log -Append -Encoding ascii
} catch {
    "ERROR: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}
