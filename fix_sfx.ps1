$fxPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{177fab71-626e-4eea-ad8f-95284cf352d6}\FxProperties"
$sfxKey = "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5"
$ourClsid = "{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}"

Set-ItemProperty $fxPath -Name $sfxKey -Value $ourClsid -Type String
Write-Host "Legacy SFX set to:" (Get-ItemProperty $fxPath -Name $sfxKey).$sfxKey

Remove-Item "C:\ProgramData\HrtfApo\debug.log" -Force -ErrorAction SilentlyContinue

net stop audiosrv /y
net stop AudioEndpointBuilder /y
Start-Sleep 1
net start AudioEndpointBuilder
net start audiosrv
Start-Sleep 3

Write-Host "Legacy SFX after restart:" (Get-ItemProperty $fxPath -Name $sfxKey).$sfxKey

# Play audio
ffplay -nodisp -autoexit -t 3 -f lavfi -i "sine=frequency=440:duration=3" 2>$null
Start-Sleep 2

if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
    Write-Host "=== debug.log ==="
    Get-Content "C:\ProgramData\HrtfApo\debug.log"
} else {
    Write-Host "No debug.log"
}
