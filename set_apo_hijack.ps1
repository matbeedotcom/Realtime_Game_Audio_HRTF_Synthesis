$log = "C:\ProgramData\HrtfApo\install.log"

try {
    "=== APO Hijack Install $(Get-Date) ===" | Out-File $log -Encoding ascii

    # The Sound Blaster's existing SFX APO CLSID
    $existingClsid = "{C9453E73-8C5C-4463-9984-AF8BAB2F5447}"
    $comPath = "HKLM:\SOFTWARE\Classes\CLSID\$existingClsid\InProcServer32"

    # Read the original DLL path first (so we can restore it later)
    $originalDll = (Get-ItemProperty $comPath -ErrorAction Stop).'(Default)'
    "Original DLL: $originalDll" | Out-File $log -Append -Encoding ascii

    # Save original for restoration
    "OriginalDll=$originalDll" | Out-File "C:\ProgramData\HrtfApo\original_apo.txt" -Encoding ascii

    # Point the existing CLSID to our DLL
    Set-ItemProperty $comPath -Name "(Default)" -Value "C:\ProgramData\HrtfApo\hrtf_apo.dll" -Force
    "Redirected $existingClsid to our DLL" | Out-File $log -Append -Encoding ascii

    # Also update our APO registration to use this CLSID
    $regPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\$existingClsid"
    if (-not (Test-Path $regPath)) {
        "APO registration path does not exist: $regPath" | Out-File $log -Append -Encoding ascii
    } else {
        "APO registration exists at: $regPath" | Out-File $log -Append -Encoding ascii
    }

    # Verify
    $after = (Get-ItemProperty $comPath).'(Default)'
    "COM DLL after: $after" | Out-File $log -Append -Encoding ascii

    # Restart audio
    net stop audiosrv 2>&1 | Out-File $log -Append -Encoding ascii
    Start-Sleep 1
    net start audiosrv 2>&1 | Out-File $log -Append -Encoding ascii

    Start-Sleep 2
    if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
        "DEBUG LOG FOUND" | Out-File $log -Append -Encoding ascii
        Get-Content "C:\ProgramData\HrtfApo\debug.log" | Out-File $log -Append -Encoding ascii
    } else {
        "No debug.log yet" | Out-File $log -Append -Encoding ascii
    }

    "Done" | Out-File $log -Append -Encoding ascii
} catch {
    "ERROR: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}
