$log = "C:\ProgramData\HrtfApo\install.log"
$existingClsid = "{C9453E73-8C5C-4463-9984-AF8BAB2F5447}"

try {
    "=== APO Hijack Part 2 $(Get-Date) ===" | Out-File $log -Encoding ascii

    # Register APO properties under the existing CLSID
    $regPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\$existingClsid"
    if (-not (Test-Path $regPath)) {
        New-Item -Path $regPath -Force | Out-Null
    }
    Set-ItemProperty $regPath -Name "FriendlyName" -Value "Personalized HRTF Binaural" -Type String
    Set-ItemProperty $regPath -Name "MajorVersion" -Value 1 -Type DWord
    Set-ItemProperty $regPath -Name "MinorVersion" -Value 0 -Type DWord
    Set-ItemProperty $regPath -Name "Flags" -Value 0x0C -Type DWord
    Set-ItemProperty $regPath -Name "MinInputConnections" -Value 1 -Type DWord
    Set-ItemProperty $regPath -Name "MaxInputConnections" -Value 1 -Type DWord
    Set-ItemProperty $regPath -Name "MinOutputConnections" -Value 1 -Type DWord
    Set-ItemProperty $regPath -Name "MaxOutputConnections" -Value 1 -Type DWord
    Set-ItemProperty $regPath -Name "MaxInstances" -Value 0xFFFFFFFF -Type DWord
    "APO registered under $existingClsid" | Out-File $log -Append -Encoding ascii

    # Verify COM redirect is still pointing to us
    $comPath = "HKLM:\SOFTWARE\Classes\CLSID\$existingClsid\InProcServer32"
    $dll = (Get-ItemProperty $comPath).'(Default)'
    "COM DLL: $dll" | Out-File $log -Append -Encoding ascii

    # Delete any old debug log
    Remove-Item "C:\ProgramData\HrtfApo\debug.log" -ErrorAction SilentlyContinue

    # Restart audio
    net stop audiosrv 2>&1 | Out-File $log -Append -Encoding ascii
    Start-Sleep 1
    net start audiosrv 2>&1 | Out-File $log -Append -Encoding ascii

    # Play a beep to force audio engine to load APOs
    [Console]::Beep(440, 500)
    Start-Sleep 2

    if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
        "DEBUG LOG FOUND:" | Out-File $log -Append -Encoding ascii
        Get-Content "C:\ProgramData\HrtfApo\debug.log" | Out-File $log -Append -Encoding ascii
    } else {
        "No debug.log" | Out-File $log -Append -Encoding ascii
    }

    "Done" | Out-File $log -Append -Encoding ascii
} catch {
    "ERROR: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}
