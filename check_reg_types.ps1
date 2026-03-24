$working = Get-Item 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{f6654c7e-35f3-4d39-b531-3c0e94f98e55}\FxProperties'
$ours = Get-Item 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{177fab71-626e-4eea-ad8f-95284cf352d6}\FxProperties'

Write-Host "=== Working endpoint ==="
Write-Host "Composite type:" $working.GetValueKind('{d3993a3f-99c2-4402-b5ec-a92a0367664b},5')
Write-Host "Composite value:" $working.GetValue('{d3993a3f-99c2-4402-b5ec-a92a0367664b},5')
Write-Host "Legacy type:" $working.GetValueKind('{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5')
Write-Host "Legacy value:" $working.GetValue('{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5')

Write-Host ""
Write-Host "=== Our endpoint ==="
Write-Host "Composite type:" $ours.GetValueKind('{d3993a3f-99c2-4402-b5ec-a92a0367664b},5')
Write-Host "Composite value:" $ours.GetValue('{d3993a3f-99c2-4402-b5ec-a92a0367664b},5')
Write-Host "Legacy type:" $ours.GetValueKind('{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5')
Write-Host "Legacy value:" $ours.GetValue('{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5')
