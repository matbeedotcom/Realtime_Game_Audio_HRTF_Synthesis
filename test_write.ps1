$regPath = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{f6654c7e-35f3-4d39-b531-3c0e94f98e55}\FxProperties"
$valueName = "{d3993a3f-99c2-4402-b5ec-a92a0367664b},5"
$ourClsid = "{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}"
$out = "C:\ProgramData\HrtfApo\write_test.txt"

try {
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($regPath, $false)
    $before = $k.GetValue($valueName)
    $kind = $k.GetValueKind($valueName)
    $k.Close()
    "Before: $($before -join ', ') (type: $kind)" | Out-File $out -Encoding ascii

    # Open with SetValue permission only
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $regPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::SetValue)

    # Build new value as proper string array
    [string[]]$newVal = @($ourClsid, "{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}")

    $k.SetValue($valueName, $newVal, [Microsoft.Win32.RegistryValueKind]::MultiString)
    $k.Close()

    # Verify
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($regPath, $false)
    $after = $k.GetValue($valueName)
    $afterKind = $k.GetValueKind($valueName)
    $k.Close()
    "After: $($after -join ', ') (type: $afterKind)" | Out-File $out -Append -Encoding ascii
    "SUCCESS" | Out-File $out -Append -Encoding ascii
} catch {
    "FAILED: $($_.Exception.Message)" | Out-File $out -Append -Encoding ascii
}
