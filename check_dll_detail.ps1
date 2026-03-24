$adg = Get-Process audiodg -ErrorAction SilentlyContinue
if (-not $adg) { "audiodg not running" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt"; exit }

$mod = $adg.Modules | Where-Object { $_.ModuleName -eq "hrtf_apo.dll" }
if ($mod) {
    "Found hrtf_apo.dll:" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Encoding ascii
    "  FileName: $($mod.FileName)" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Append -Encoding ascii
    "  Size: $($mod.ModuleMemorySize)" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Append -Encoding ascii
    "  Base: $($mod.BaseAddress)" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Append -Encoding ascii
    "  FileVersion: $($mod.FileVersionInfo.FileVersion)" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Append -Encoding ascii

    # Check file hash of loaded vs on-disk
    $diskHash = (Get-FileHash $mod.FileName).Hash
    $builtHash = (Get-FileHash "C:\Users\mail\dev\personal\Individualized_HRTF_Synthesis\target\release\hrtf_apo.dll").Hash
    "  Disk hash: $diskHash" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Append -Encoding ascii
    "  Built hash: $builtHash" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Append -Encoding ascii
    "  Match: $($diskHash -eq $builtHash)" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Append -Encoding ascii
} else {
    "hrtf_apo.dll NOT loaded" | Out-File "C:\ProgramData\HrtfApo\dllinfo.txt" -Encoding ascii
}
