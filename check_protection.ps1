# Play audio to force audiodg to load APOs
$ffplay = Get-Command ffplay -ErrorAction SilentlyContinue
if ($ffplay) {
    $proc = Start-Process ffplay -ArgumentList '-nodisp','-autoexit','-t','3','-f','lavfi','-i','sine=frequency=440:duration=3' -PassThru
    Start-Sleep 2
} else {
    # Fallback: play a system sound
    [System.Media.SystemSounds]::Asterisk.Play()
    Start-Sleep 2
}

$out = "C:\ProgramData\HrtfApo\procinfo.txt"

$adg = Get-Process audiodg -ErrorAction SilentlyContinue
"audiodg PID: $($adg.Id)" | Out-File $out -Encoding ascii
"Start: $($adg.StartTime)" | Out-File $out -Append -Encoding ascii

# Check command line
$wmi = Get-CimInstance Win32_Process -Filter "ProcessId=$($adg.Id)"
"CommandLine: $($wmi.CommandLine)" | Out-File $out -Append -Encoding ascii

# List modules
"" | Out-File $out -Append -Encoding ascii
"Modules:" | Out-File $out -Append -Encoding ascii
$adg.Modules | ForEach-Object {
    "  $($_.ModuleName) - $($_.FileName) ($($_.ModuleMemorySize))" | Out-File $out -Append -Encoding ascii
}

# Check file hash of the WMALFXGFXDSP.dll that's loaded
$wma = $adg.Modules | Where-Object { $_.ModuleName -eq "WMALFXGFXDSP.dll" }
if ($wma) {
    "" | Out-File $out -Append -Encoding ascii
    "WMALFXGFXDSP.dll loaded from: $($wma.FileName)" | Out-File $out -Append -Encoding ascii
    "Memory size: $($wma.ModuleMemorySize)" | Out-File $out -Append -Encoding ascii
}

# Check WinSxS cache
$winsxs = Get-ChildItem "C:\Windows\WinSxS" -Filter "WMALFXGFXDSP.dll" -Recurse -ErrorAction SilentlyContinue
if ($winsxs) {
    "" | Out-File $out -Append -Encoding ascii
    "WinSxS copies:" | Out-File $out -Append -Encoding ascii
    $winsxs | ForEach-Object { "  $($_.FullName)" | Out-File $out -Append -Encoding ascii }
}
