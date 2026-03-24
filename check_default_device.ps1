# Check which device is the default render endpoint
# and whether our CLSID is on it

$renderGUIDs = @(
    "{177fab71-626e-4eea-ad8f-95284cf352d6}",
    "{47964a7a-59f5-4390-a76a-53a6f8cd031d}",
    "{562ddaca-13ed-47d1-8412-91daa06e8588}",
    "{5db32094-2e0a-4a13-83eb-5d55579672ce}",
    "{a5a7c494-4503-45b8-87e0-f98753665bc1}",
    "{c0eb31df-9ce4-42cd-bd70-e592cd322c2e}",
    "{d1bccb8d-d2c0-4502-abda-8e70e1830d75}",
    "{f6654c7e-35f3-4d39-b531-3c0e94f98e55}"
)

$basePath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render"

foreach ($guid in $renderGUIDs) {
    $devPath = Join-Path $basePath $guid
    $propsPath = Join-Path $devPath "Properties"
    $fxPath = Join-Path $devPath "FxProperties"

    $state = (Get-ItemProperty $devPath -Name 'DeviceState' -ErrorAction SilentlyContinue).DeviceState
    if ($state -ne 1) { continue }

    # Get friendly name from Properties
    $name = "Unknown"
    try {
        $name = (Get-ItemProperty $propsPath -ErrorAction SilentlyContinue).'{b725f130-47ef-101a-a5f1-02608c9eebac},6'
        if (-not $name) {
            $name = (Get-ItemProperty $propsPath -ErrorAction SilentlyContinue).'{a45c254e-df1c-4efd-8020-67d146a850e0},2'
        }
    } catch {}

    # Get SFX CLSIDs
    $legacySfx = ""
    $compositeSfx = ""
    try {
        $fxProps = Get-ItemProperty $fxPath -ErrorAction SilentlyContinue
        $legacySfx = $fxProps.'{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5'
        $compositeSfx = $fxProps.'{d3993a3f-99c2-4402-b5ec-a92a0367664b},5'
    } catch {}

    $hasOurs = ($legacySfx -like "*A1B2C3D4*") -or ($compositeSfx -like "*A1B2C3D4*")
    $marker = if ($hasOurs) { " <-- OUR APO" } else { "" }

    Write-Host "$guid - $name$marker"
    Write-Host "  Legacy SFX:    $legacySfx"
    Write-Host "  Composite SFX: $compositeSfx"
    Write-Host ""
}
