# Full APO deploy: build, register, install, play audio, show results
# Must be run as Administrator
param([switch]$SkipBuild)

$log = "C:\ProgramData\HrtfApo\deploy.log"
$projDir = "C:\Users\mail\dev\personal\Individualized_HRTF_Synthesis"
$endpoint = "{f6654c7e-35f3-4d39-b531-3c0e94f98e55}"
$ourClsid = "{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}"
$dllSrc = "$projDir\target\release\hrtf_apo.dll"
$dllDst = "C:\ProgramData\HrtfApo\hrtf_apo.dll"
$endpointRegPath = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$endpoint"
$fxRegPath = "$endpointRegPath\FxProperties"
$fxPath = "HKLM:\$fxRegPath"
$compSfxKey = "{d3993a3f-99c2-4402-b5ec-a92a0367664b},5"

function Log($msg) {
    $ts = Get-Date -Format "HH:mm:ss"
    "[$ts] $msg" | Out-File $log -Append -Encoding ascii
    Write-Host "[$ts] $msg"
}

"=== APO Deploy $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') ===" | Out-File $log -Encoding ascii

# ── 1. Build ─────────────────────────────────────────────────────────────
if (!$SkipBuild) {
    Log "Building hrtf-apo..."
    $buildOut = & cargo build --release -p hrtf-apo --manifest-path "$projDir\Cargo.toml" 2>&1
    if ($LASTEXITCODE -ne 0) {
        Log "BUILD FAILED:"
        $buildOut | Out-File $log -Append -Encoding ascii
        Write-Host ($buildOut -join "`n") -ForegroundColor Red
        exit 1
    }
    Log "Build OK"
} else {
    Log "Skipping build"
}

# ── 2. Clean unregister + stop services ──────────────────────────────────
Log "Unregistering old APO..."
regsvr32 /u /s $dllDst 2>&1 | Out-Null

Log "Stopping audio services..."
net stop audiosrv /y 2>&1 | Out-Null
net stop AudioEndpointBuilder /y 2>&1 | Out-Null
Start-Sleep 1

$adg = Get-Process audiodg -ErrorAction SilentlyContinue
if ($adg) {
    Stop-Process -Force -Id $adg.Id -ErrorAction SilentlyContinue
    Start-Sleep 1
}

# Clear debug log
Remove-Item "C:\ProgramData\HrtfApo\debug.log" -ErrorAction SilentlyContinue

# ── 3. Copy DLL to ProgramData AND System32 ──────────────────────────────
$dllSys32 = "C:\WINDOWS\System32\hrtf_apo.dll"
try {
    Copy-Item $dllSrc $dllDst -Force
    Copy-Item $dllSrc $dllSys32 -Force
    Log "DLL copied to ProgramData + System32"
} catch {
    Log "DLL copy FAILED: $($_.Exception.Message)"
    exit 1
}

# ── 4. COM + APO registration ────────────────────────────────────────────
$comPath = "HKLM:\SOFTWARE\Classes\CLSID\$ourClsid\InProcServer32"
if (!(Test-Path $comPath)) { New-Item $comPath -Force | Out-Null }
Set-ItemProperty $comPath -Name "(Default)" -Value $dllSys32
Set-ItemProperty $comPath -Name "ThreadingModel" -Value "Both"

# Register APO via DllRegisterServer (calls RegisterAPO from Windows SDK)
# This properly writes all required registry entries
$regout = regsvr32 /s $dllSys32 2>&1
Log "regsvr32: $regout"
# Ensure COM InProcServer32 points to System32 copy
$comPath2 = "HKLM:\SOFTWARE\Classes\CLSID\$ourClsid\InProcServer32"
Set-ItemProperty $comPath2 -Name "(Default)" -Value $dllSys32 -ErrorAction SilentlyContinue
Log "COM + APO registered via RegisterAPO"

# ── 5. Install for endpoint via native C++ (EqualizerAPO-style takeOwnership + FxProperties write)
Remove-Item "C:\ProgramData\HrtfApo\debug.log" -ErrorAction SilentlyContinue
Log "Running InstallForEndpoint via rundll32..."
$installOut = & rundll32.exe $dllSys32,InstallForEndpoint $endpoint 2>&1
Start-Sleep 2
if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
    Get-Content "C:\ProgramData\HrtfApo\debug.log" | ForEach-Object { Log "  $_" }
}

# ── 5old. Write FxProperties per EqualizerAPO Developer.txt step 4 ──────────
# Legacy SFX key ({d04e05a6-...},5) = our CLSID
# Processing mode key ({d3993a3f-...},5) = default processing mode GUID
$legacySfxKey = "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5"
$procModeKey = "{d3993a3f-99c2-4402-b5ec-a92a0367664b},5"
$defaultProcMode = "{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}"
try {
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($fxRegPath, $false)
    $legacyBefore = $k.GetValue($legacySfxKey)
    $procModeBefore = $k.GetValue($procModeKey)
    $k.Close()
    Log "Legacy SFX before: $legacyBefore"
    Log "Processing mode before: $($procModeBefore -join ', ')"

    # Save original SFX CLSID for restore
    if ($legacyBefore -and $legacyBefore -ne $ourClsid) {
        $childApoPath = "HKLM:\SOFTWARE\HrtfApo\ChildAPOs\$endpoint"
        if (!(Test-Path $childApoPath)) { New-Item $childApoPath -Force | Out-Null }
        Set-ItemProperty $childApoPath -Name "OriginalSFX" -Value $legacyBefore -Type String
        Log "Saved original SFX: $legacyBefore"
    }

    # Write our CLSID to legacy SFX key
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $fxRegPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::SetValue -bor [System.Security.AccessControl.RegistryRights]::QueryValues)
    $k.SetValue($legacySfxKey, $ourClsid, [Microsoft.Win32.RegistryValueKind]::String)

    # Fix processing mode: ensure ONLY the default processing mode is set (NOT our CLSID!)
    try { $k.DeleteValue($procModeKey, $false) } catch {}
    [string[]]$procModeArr = @($defaultProcMode)
    $k.SetValue($procModeKey, $procModeArr, [Microsoft.Win32.RegistryValueKind]::MultiString)
    $k.Close()

    # Verify
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($fxRegPath, $false)
    $legacyAfter = $k.GetValue($legacySfxKey)
    $procModeAfter = $k.GetValue($procModeKey)
    $k.Close()
    Log "Legacy SFX after: $legacyAfter"
    Log "Processing mode after: $($procModeAfter -join ', ')"
} catch {
    Log "FxProperties write FAILED: $($_.Exception.Message)"
}

# ── 5b. Write composite SFX with our CLSID ──────────────────────────────
try {
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $fxRegPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::SetValue -bor [System.Security.AccessControl.RegistryRights]::QueryValues)
    [string[]]$compVal = @($ourClsid, $defaultProcMode)
    $k.SetValue($compSfxKey, $compVal, [Microsoft.Win32.RegistryValueKind]::MultiString)
    $k.Close()
    Log "Composite SFX set to: $($compVal -join ', ')"
} catch {
    Log "Composite SFX write FAILED: $($_.Exception.Message)"
}

# ── 5c. Remove AudioEndpointBuilder write access to prevent reset ────────
try {
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $fxRegPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::ChangePermissions -bor [System.Security.AccessControl.RegistryRights]::ReadPermissions)
    $acl = $k.GetAccessControl()
    $aebSid = New-Object System.Security.Principal.NTAccount("NT SERVICE\AudioEndpointBuilder")
    # Add deny-write rule for AudioEndpointBuilder
    $denyRule = New-Object System.Security.AccessControl.RegistryAccessRule(
        $aebSid,
        [System.Security.AccessControl.RegistryRights]::SetValue,
        [System.Security.AccessControl.AccessControlType]::Deny)
    $acl.AddAccessRule($denyRule)
    $k.SetAccessControl($acl)
    $k.Close()
    Log "Denied AudioEndpointBuilder write access to FxProperties"
} catch {
    Log "ACL deny FAILED: $($_.Exception.Message)"
}

# ── 5d. Delete DisableEnhancements if present ────────────────────────────
try {
    $disableKey = "{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5"
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($fxRegPath, $false)
    $val = $k.GetValue($disableKey)
    $k.Close()
    if ($val -ne $null) {
        $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
            $fxRegPath,
            [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
            [System.Security.AccessControl.RegistryRights]::SetValue)
        $k.DeleteValue($disableKey, $false)
        $k.Close()
        Log "Deleted DisableEnhancements"
    }
} catch {}

# ── 6c. Create startup task to re-apply composite SFX after AudioEndpointBuilder resets it
$startupScript = @"
# HrtfApo startup script — re-applies composite SFX CLSID after AudioEndpointBuilder resets it
Start-Sleep 10  # Wait for AudioEndpointBuilder to finish resetting

`$fxPath = 'HKLM:\$fxRegPath'
`$compKey = '$compSfxKey'
`$ourClsid = '$ourClsid'

`$current = (Get-ItemProperty `$fxPath -Name `$compKey -ErrorAction SilentlyContinue).`$compKey
if (`$current -is [string[]]) {
    if (`$current -notcontains `$ourClsid) {
        `$newVal = @(`$ourClsid) + `$current
    } else { return }
} elseif (`$current -is [string]) {
    if (`$current -ne `$ourClsid) {
        `$newVal = @(`$ourClsid, `$current)
    } else { return }
} else {
    `$newVal = @(`$ourClsid)
}

try {
    `$k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        '$fxRegPath',
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::SetValue -bor [System.Security.AccessControl.RegistryRights]::QueryValues)
    `$k.SetValue(`$compKey, `$newVal, [Microsoft.Win32.RegistryValueKind]::MultiString)
    `$k.Close()

    # Restart audio service to pick up the change
    net stop audiosrv /y 2>&1 | Out-Null
    Start-Sleep 1
    net start audiosrv 2>&1 | Out-Null
} catch {}
"@

$startupScript | Out-File "C:\ProgramData\HrtfApo\startup.ps1" -Encoding ascii
schtasks /Create /TN "HrtfApoStartup" /TR "powershell -ExecutionPolicy Bypass -WindowStyle Hidden -File C:\ProgramData\HrtfApo\startup.ps1" /SC ONSTART /RU SYSTEM /F 2>&1 | Out-Null
Log "Created startup scheduled task"

# ── 7. Restart audio ─────────────────────────────────────────────────────
Remove-Item "C:\ProgramData\HrtfApo\debug.log" -ErrorAction SilentlyContinue
net start AudioEndpointBuilder 2>&1 | Out-Null
net start audiosrv 2>&1 | Out-Null
Start-Sleep 3

# Now fix processing mode (we may have contaminated it earlier) and re-apply legacy SFX
try {
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $fxRegPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::SetValue -bor [System.Security.AccessControl.RegistryRights]::QueryValues)
    # Processing mode must be ONLY the default mode GUID, not our CLSID
    [string[]]$pmArr = @($defaultProcMode)
    $k.SetValue($procModeKey, $pmArr, [Microsoft.Win32.RegistryValueKind]::MultiString)
    # Re-apply legacy SFX CLSID
    $k.SetValue($legacySfxKey, $ourClsid, [Microsoft.Win32.RegistryValueKind]::String)
    $k.Close()
    Log "Fixed processing mode + re-applied legacy SFX after service restart"

    # Restart audiosrv again to pick up the change
    net stop audiosrv /y 2>&1 | Out-Null
    Start-Sleep 1
    net start audiosrv 2>&1 | Out-Null
    Start-Sleep 2
} catch {
    Log "Composite re-apply FAILED: $($_.Exception.Message)"
}

# Verify
$verifyLegacy = (Get-ItemProperty $fxPath -Name $legacySfxKey -ErrorAction SilentlyContinue).$legacySfxKey
$verifyComp = (Get-ItemProperty $fxPath -Name $compSfxKey -ErrorAction SilentlyContinue).$compSfxKey
$verifyProcMode = (Get-ItemProperty $fxPath -Name $procModeKey -ErrorAction SilentlyContinue).$procModeKey
Log "Legacy SFX after restart: $verifyLegacy"
Log "Composite SFX after restart: $($verifyComp -join ', ')"
Log "Processing mode after restart: $($verifyProcMode -join ', ')"

# ── 8. Play audio to trigger APO loading ──────────────────────────────────
Log "Waiting for audio engine to settle..."
Start-Sleep 5
Log "Playing test tone..."
Start-Process ffplay -ArgumentList '-nodisp','-autoexit','-t','5','-f','lavfi','-i','sine=frequency=440:duration=5' -Wait -ErrorAction SilentlyContinue
Start-Sleep 3

# ── 9. Check results ─────────────────────────────────────────────────────
$adg = Get-Process audiodg -ErrorAction SilentlyContinue
if ($adg) {
    Log "audiodg PID: $($adg.Id)"
    try {
        $ourDll = $adg.Modules | Where-Object { $_.ModuleName -eq "hrtf_apo.dll" }
        if ($ourDll) { Log "hrtf_apo.dll LOADED: $($ourDll.FileName)" }
        else { Log "hrtf_apo.dll NOT loaded" }
    } catch {
        Log "Module check needs elevation"
    }
}

if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
    Log "=== debug.log ==="
    Get-Content "C:\ProgramData\HrtfApo\debug.log" | ForEach-Object { Log $_ }
} else {
    Log "No debug.log"
}

Log "=== DONE ==="
