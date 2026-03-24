$basePath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render"
Get-ChildItem $basePath | ForEach-Object {
    $guid = $_.PSChildName
    $state = (Get-ItemProperty $_.PSPath -Name 'DeviceState' -ErrorAction SilentlyContinue).DeviceState
    $stateStr = switch ($state) { 1 {"ACTIVE"} 4 {"NOTPRESENT"} 8 {"UNPLUGGED"} default {"state=$state"} }

    $propsPath = Join-Path $_.PSPath "Properties"
    $name = ""
    try {
        $name = (Get-ItemProperty $propsPath -ErrorAction SilentlyContinue).'{b725f130-47ef-101a-a5f1-02608c9eebac},6'
        if (-not $name) { $name = (Get-ItemProperty $propsPath -ErrorAction SilentlyContinue).'{a45c254e-df1c-4efd-8020-67d146a850e0},2' }
    } catch {}

    $fxPath = Join-Path $_.PSPath "FxProperties"
    $sfx = ""
    try { $sfx = (Get-ItemProperty $fxPath -ErrorAction SilentlyContinue).'{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5' } catch {}

    if ($state -eq 1) {
        $hasOurs = $sfx -like "*A1B2C3D4*"
        $marker = if ($hasOurs) { " *** OUR APO ***" } else { "" }
        Write-Host "$guid [$stateStr] $name (SFX: $sfx)$marker"
    }
}
