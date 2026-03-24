$log = "C:\ProgramData\HrtfApo\restore.log"
"=== Full Restore $(Get-Date) ===" | Out-File $log -Encoding ascii

net stop audiosrv 2>&1 | Out-File $log -Append -Encoding ascii
net stop AudioEndpointBuilder 2>&1 | Out-File $log -Append -Encoding ascii
Start-Sleep 1

# Restore COM CLSIDs
reg add "HKLM\SOFTWARE\Classes\CLSID\{C9453E73-8C5C-4463-9984-AF8BAB2F5447}\InProcServer32" /ve /t REG_SZ /d "C:\WINDOWS\System32\WMALFXGFXDSP.dll" /f 2>&1 | Out-File $log -Append -Encoding ascii
reg add "HKLM\SOFTWARE\Classes\CLSID\{637C490D-EEE3-4C0A-973F-371958802DA2}\InProcServer32" /ve /t REG_SZ /d "C:\WINDOWS\System32\WMALFXGFXDSP.dll" /f 2>&1 | Out-File $log -Append -Encoding ascii

# Take ownership and restore FxProperties for endpoint {177fab71-...}
$fxRegPath = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{177fab71-626e-4eea-ad8f-95284cf352d6}\FxProperties"
try {
    # Enable SeTakeOwnershipPrivilege
    $import = '[DllImport("ntdll.dll")] public static extern int RtlAdjustPrivilege(int Privilege, bool Enable, bool CurrentThread, out bool Enabled);'
    $ntdll = Add-Type -MemberDefinition $import -Name NtDll -PassThru
    [bool]$prev = $false
    $ntdll::RtlAdjustPrivilege(9, $true, $false, [ref]$prev) | Out-Null  # SeTakeOwnershipPrivilege
    $ntdll::RtlAdjustPrivilege(17, $true, $false, [ref]$prev) | Out-Null # SeRestorePrivilege

    $key = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($fxRegPath, [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree, [System.Security.AccessControl.RegistryRights]::TakeOwnership)
    $acl = $key.GetAccessControl()
    $admin = New-Object System.Security.Principal.NTAccount("BUILTIN", "Administrators")
    $acl.SetOwner($admin)
    $key.SetAccessControl($acl)
    "Took ownership" | Out-File $log -Append -Encoding ascii

    # Now grant full control
    $acl = $key.GetAccessControl()
    $rule = New-Object System.Security.AccessControl.RegistryAccessRule($admin, "FullControl", "ContainerInherit,ObjectInherit", "None", "Allow")
    $acl.AddAccessRule($rule)
    $key.SetAccessControl($acl)
    $key.Close()
    "Granted full control" | Out-File $log -Append -Encoding ascii

    # Now write the values
    Set-ItemProperty "HKLM:\$fxRegPath" -Name '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5' -Value '{71DAB6A1-39F3-423E-90A8-032729851157}' -Type String
    Set-ItemProperty "HKLM:\$fxRegPath" -Name '{d3993a3f-99c2-4402-b5ec-a92a0367664b},5' -Value @('{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}') -Type MultiString
    "FxProperties restored" | Out-File $log -Append -Encoding ascii
} catch {
    "FxProperties error: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}

# Verify
$val = (Get-ItemProperty "HKLM:\$fxRegPath")."{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5"
"Endpoint 177fab71 Legacy SFX: $val" | Out-File $log -Append -Encoding ascii

# Restore System32 DLL if needed
$sysHash = (Get-FileHash "C:\WINDOWS\System32\WMALFXGFXDSP.dll").Hash
$bakHash = (Get-FileHash "C:\ProgramData\HrtfApo\WMALFXGFXDSP.dll.bak").Hash
if ($sysHash -ne $bakHash) {
    Copy-Item "C:\ProgramData\HrtfApo\WMALFXGFXDSP.dll.bak" "C:\WINDOWS\System32\WMALFXGFXDSP.dll" -Force
    "System32 DLL restored" | Out-File $log -Append -Encoding ascii
} else {
    "System32 DLL already original" | Out-File $log -Append -Encoding ascii
}

net start AudioEndpointBuilder 2>&1 | Out-File $log -Append -Encoding ascii
net start audiosrv 2>&1 | Out-File $log -Append -Encoding ascii
"Done" | Out-File $log -Append -Encoding ascii
