$path = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{f6654c7e-35f3-4d39-b531-3c0e94f98e55}\FxProperties"
$out = "C:\ProgramData\HrtfApo\acl.txt"

try {
    $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($path, $false)
    $acl = $k.GetAccessControl()

    "Owner: $($acl.GetOwner([System.Security.Principal.NTAccount]))" | Out-File $out -Encoding ascii
    "" | Out-File $out -Append -Encoding ascii

    foreach ($rule in $acl.GetAccessRules($true, $true, [System.Security.Principal.NTAccount])) {
        "Identity: $($rule.IdentityReference)" | Out-File $out -Append -Encoding ascii
        "  Type: $($rule.AccessControlType)" | Out-File $out -Append -Encoding ascii
        "  Rights: $($rule.RegistryRights)" | Out-File $out -Append -Encoding ascii
        "  Inherit: $($rule.InheritanceFlags)" | Out-File $out -Append -Encoding ascii
        "" | Out-File $out -Append -Encoding ascii
    }
    $k.Close()
    "Done" | Out-File $out -Append -Encoding ascii
} catch {
    "ERROR: $($_.Exception.Message)" | Out-File $out -Encoding ascii
}
