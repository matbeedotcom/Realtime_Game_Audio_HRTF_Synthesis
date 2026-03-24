$log = "C:\ProgramData\HrtfApo\install.log"
$clsid = "{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}"
$regPath = "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{f6654c7e-35f3-4d39-b531-3c0e94f98e55}\FxProperties"

try {
    "=== APO Install $(Get-Date) ===" | Out-File $log -Encoding ascii

    # Use a scheduled task running as SYSTEM to write the registry values
    # SYSTEM has full access to MMDevices keys
    $taskAction = New-ScheduledTaskAction -Execute "reg.exe" -Argument "add `"$regPath`" /v `"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5`" /t REG_SZ /d `"$clsid`" /f"
    $taskPrincipal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest
    $task = New-ScheduledTask -Action $taskAction -Principal $taskPrincipal
    Register-ScheduledTask -TaskName "HrtfApoSetSfx" -InputObject $task -Force | Out-Null
    Start-ScheduledTask -TaskName "HrtfApoSetSfx"
    Start-Sleep 1
    Unregister-ScheduledTask -TaskName "HrtfApoSetSfx" -Confirm:$false
    "Legacy SFX written via SYSTEM task" | Out-File $log -Append -Encoding ascii

    # Now write the composite (REG_MULTI_SZ) — reg.exe uses \0 separator
    $taskAction = New-ScheduledTaskAction -Execute "reg.exe" -Argument "add `"$regPath`" /v `"{d3993a3f-99c2-4402-b5ec-a92a0367664b},5`" /t REG_MULTI_SZ /d `"$clsid`" /f"
    $task = New-ScheduledTask -Action $taskAction -Principal $taskPrincipal
    Register-ScheduledTask -TaskName "HrtfApoSetSfxComp" -InputObject $task -Force | Out-Null
    Start-ScheduledTask -TaskName "HrtfApoSetSfxComp"
    Start-Sleep 1
    Unregister-ScheduledTask -TaskName "HrtfApoSetSfxComp" -Confirm:$false
    "Composite SFX written via SYSTEM task" | Out-File $log -Append -Encoding ascii

    # Verify
    $after = (Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{f6654c7e-35f3-4d39-b531-3c0e94f98e55}\FxProperties").'{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5'
    $afterComp = (Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{f6654c7e-35f3-4d39-b531-3c0e94f98e55}\FxProperties").'{d3993a3f-99c2-4402-b5ec-a92a0367664b},5'
    "Legacy SFX after: $after" | Out-File $log -Append -Encoding ascii
    "Composite SFX after: $afterComp" | Out-File $log -Append -Encoding ascii

    # Restart audio
    net stop audiosrv 2>&1 | Out-File $log -Append -Encoding ascii
    Start-Sleep 1
    net start audiosrv 2>&1 | Out-File $log -Append -Encoding ascii
    "Done" | Out-File $log -Append -Encoding ascii
} catch {
    "ERROR: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}
