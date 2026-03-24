# Must run as admin to read audiodg modules
$proc = Get-Process audiodg -ErrorAction SilentlyContinue
if ($null -eq $proc) {
    Write-Host "audiodg.exe not running"
    exit
}

Write-Host "audiodg.exe PID: $($proc.Id), started: $($proc.StartTime)"
Write-Host ""
Write-Host "Loaded modules:"
$proc.Modules | ForEach-Object { "  $($_.ModuleName) - $($_.FileName)" } | Out-File "C:\ProgramData\HrtfApo\modules2.txt" -Encoding ascii
Write-Host "Written to modules2.txt"
