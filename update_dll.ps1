# Stop ALL audio services that might lock the DLL
"Stopping audio services..." | Out-File "C:\ProgramData\HrtfApo\update.log" -Encoding ascii
net stop audiosrv /y 2>&1 | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
net stop AudioEndpointBuilder /y 2>&1 | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
Start-Sleep 2

# Kill audiodg if still running
$adg = Get-Process audiodg -ErrorAction SilentlyContinue
if ($adg) {
    "Killing audiodg.exe (PID $($adg.Id))..." | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
    Stop-Process -Force -Id $adg.Id -ErrorAction SilentlyContinue
    Start-Sleep 1
}

# Copy the DLL to both ProgramData and System32
try {
    $src = "C:\Users\mail\dev\personal\Individualized_HRTF_Synthesis\target\release\hrtf_apo.dll"
    Copy-Item $src "C:\ProgramData\HrtfApo\hrtf_apo.dll" -Force
    Copy-Item $src "C:\WINDOWS\System32\WMALFXGFXDSP.dll" -Force
    "DLL copied to ProgramData + System32" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
} catch {
    "DLL copy FAILED: $($_.Exception.Message)" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
}

# Delete old debug log
Remove-Item "C:\ProgramData\HrtfApo\debug.log" -ErrorAction SilentlyContinue

# Restart audio
net start AudioEndpointBuilder 2>&1 | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
net start audiosrv 2>&1 | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii

# Verify hashes
$built = (Get-FileHash "C:\Users\mail\dev\personal\Individualized_HRTF_Synthesis\target\release\hrtf_apo.dll").Hash
$pd = (Get-FileHash "C:\ProgramData\HrtfApo\hrtf_apo.dll").Hash
$sys = (Get-FileHash "C:\WINDOWS\System32\WMALFXGFXDSP.dll").Hash
"Hash - built:   $built" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
"Hash - PD:      $pd" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
"Hash - System32: $sys" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
$match = if ($built -eq $sys) { "MATCH" } else { "MISMATCH" }
"System32 hash check: $match" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii

# Play audio via ffplay to force APO loading
"Playing test tone via ffplay..." | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
Start-Process ffplay -ArgumentList '-nodisp','-autoexit','-t','3','-f','lavfi','-i','sine=frequency=440:duration=3' -Wait -ErrorAction SilentlyContinue
Start-Sleep 1

# Check if our DLL is loaded
$adg = Get-Process audiodg -ErrorAction SilentlyContinue
if ($adg) {
    "audiodg PID: $($adg.Id)" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
    try {
        $wma = $adg.Modules | Where-Object { $_.ModuleName -eq "WMALFXGFXDSP.dll" }
        if ($wma) {
            "WMALFXGFXDSP.dll loaded from: $($wma.FileName)" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
        } else {
            "WMALFXGFXDSP.dll NOT loaded in audiodg" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
        }
    } catch {
        "Could not check modules: $($_.Exception.Message)" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
    }
} else {
    "audiodg not running" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
}

if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
    "=== debug.log ===" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
    Get-Content "C:\ProgramData\HrtfApo\debug.log" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
} else {
    "No debug.log after restart" | Out-File "C:\ProgramData\HrtfApo\update.log" -Append -Encoding ascii
}
