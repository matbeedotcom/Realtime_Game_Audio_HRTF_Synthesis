# Install HRTF APO — uses EqualizerAPO's technique: take ownership + set ACL + write
# Must be run as Administrator
$log = "C:\ProgramData\HrtfApo\install.log"
$endpoint = "{177fab71-626e-4eea-ad8f-95284cf352d6}"
$ourClsid = "{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}"
$dllPath = "C:\WINDOWS\System32\hrtf_apo.dll"

"=== APO Install $(Get-Date -Format 'MM/dd/yyyy HH:mm:ss') ===" | Out-File $log -Encoding ascii

# Step 1: COM registration
$comPath = "HKLM:\SOFTWARE\Classes\CLSID\$ourClsid\InProcServer32"
if (!(Test-Path $comPath)) { New-Item $comPath -Force | Out-Null }
Set-ItemProperty $comPath -Name "(Default)" -Value $dllPath
Set-ItemProperty $comPath -Name "ThreadingModel" -Value "Both"
"COM registered" | Out-File $log -Append -Encoding ascii

# Step 2: APO registration
$apoPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\$ourClsid"
if (!(Test-Path $apoPath)) { New-Item $apoPath -Force | Out-Null }
Set-ItemProperty $apoPath -Name "FriendlyName" -Value "Personalized HRTF Binaural" -Type String
Set-ItemProperty $apoPath -Name "Copyright" -Value "Copyright (C) 2026" -Type String
Set-ItemProperty $apoPath -Name "MajorVersion" -Value 1 -Type DWord
Set-ItemProperty $apoPath -Name "MinorVersion" -Value 0 -Type DWord
Set-ItemProperty $apoPath -Name "Flags" -Value 0x1D -Type DWord
Set-ItemProperty $apoPath -Name "MinInputConnections" -Value 1 -Type DWord
Set-ItemProperty $apoPath -Name "MaxInputConnections" -Value 1 -Type DWord
Set-ItemProperty $apoPath -Name "MinOutputConnections" -Value 1 -Type DWord
Set-ItemProperty $apoPath -Name "MaxOutputConnections" -Value 1 -Type DWord
Set-ItemProperty $apoPath -Name "MaxInstances" -Value 0xFFFFFFFF -Type DWord
"APO registered" | Out-File $log -Append -Encoding ascii

# Step 3: Take ownership of endpoint key + set ACL (EqualizerAPO technique)
$endpointRegPath = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$endpoint"

# Enable SE_TAKE_OWNERSHIP_NAME privilege
$definition = @'
using System;
using System.Runtime.InteropServices;

public class TokenPrivilege {
    [DllImport("advapi32.dll", SetLastError=true)]
    static extern bool OpenProcessToken(IntPtr ProcessHandle, uint DesiredAccess, out IntPtr TokenHandle);
    [DllImport("advapi32.dll", SetLastError=true)]
    static extern bool LookupPrivilegeValue(string lpSystemName, string lpName, out long lpLuid);
    [DllImport("advapi32.dll", SetLastError=true)]
    static extern bool AdjustTokenPrivileges(IntPtr TokenHandle, bool DisableAllPrivileges, ref TOKEN_PRIVILEGES NewState, int BufferLength, IntPtr PreviousState, IntPtr ReturnLength);
    [DllImport("kernel32.dll")]
    static extern IntPtr GetCurrentProcess();

    [StructLayout(LayoutKind.Sequential)]
    struct TOKEN_PRIVILEGES {
        public int PrivilegeCount;
        public long Luid;
        public int Attributes;
    }

    public static void Enable(string privilege) {
        IntPtr token;
        OpenProcessToken(GetCurrentProcess(), 0x0028, out token); // TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY
        TOKEN_PRIVILEGES tp = new TOKEN_PRIVILEGES();
        tp.PrivilegeCount = 1;
        tp.Attributes = 2; // SE_PRIVILEGE_ENABLED
        LookupPrivilegeValue(null, privilege, out tp.Luid);
        AdjustTokenPrivileges(token, false, ref tp, 0, IntPtr.Zero, IntPtr.Zero);
    }
}
'@

try {
    Add-Type -TypeDefinition $definition -ErrorAction SilentlyContinue
} catch {}

[TokenPrivilege]::Enable("SeTakeOwnershipPrivilege")
"Enabled SeTakeOwnershipPrivilege" | Out-File $log -Append -Encoding ascii

# Take ownership of the endpoint key (parent of FxProperties)
try {
    $regKey = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $endpointRegPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::TakeOwnership
    )
    $acl = $regKey.GetAccessControl()
    $adminsSid = New-Object System.Security.Principal.SecurityIdentifier("S-1-5-32-544") # Administrators
    $acl.SetOwner($adminsSid)
    $regKey.SetAccessControl($acl)
    $regKey.Close()
    "Took ownership of endpoint key" | Out-File $log -Append -Encoding ascii
} catch {
    "Failed to take ownership: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}

# Grant Administrators full control on endpoint key and subkeys
try {
    $regKey = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $endpointRegPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::ChangePermissions
    )
    $acl = $regKey.GetAccessControl()
    $adminsSid = New-Object System.Security.Principal.SecurityIdentifier("S-1-5-32-544")
    $rule = New-Object System.Security.AccessControl.RegistryAccessRule(
        $adminsSid,
        [System.Security.AccessControl.RegistryRights]::FullControl,
        ([System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor [System.Security.AccessControl.InheritanceFlags]::ObjectInherit),
        [System.Security.AccessControl.PropagationFlags]::None,
        [System.Security.AccessControl.AccessControlType]::Allow
    )
    $acl.AddAccessRule($rule)
    $regKey.SetAccessControl($acl)
    $regKey.Close()
    "Set Administrators ACL on endpoint key" | Out-File $log -Append -Encoding ascii
} catch {
    "Failed to set ACL: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}

# Also take ownership and set ACL on FxProperties subkey specifically
$fxRegPath = "$endpointRegPath\FxProperties"
try {
    $regKey = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $fxRegPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::TakeOwnership
    )
    $acl = $regKey.GetAccessControl()
    $acl.SetOwner($adminsSid)
    $regKey.SetAccessControl($acl)
    $regKey.Close()

    $regKey = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(
        $fxRegPath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree,
        [System.Security.AccessControl.RegistryRights]::ChangePermissions
    )
    $acl = $regKey.GetAccessControl()
    $acl.AddAccessRule($rule)
    $regKey.SetAccessControl($acl)
    $regKey.Close()
    "Set Administrators ACL on FxProperties key" | Out-File $log -Append -Encoding ascii
} catch {
    "Failed FxProperties ACL: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}

# Step 4: Write our CLSID to CORRECT composite SFX key AND processing modes
$fxPath = "HKLM:\$fxRegPath"

# PKEY_CompositeFX_StreamEffectClsid = {d04e05a6-...},13  (the CLSID list)
$compSfxClsidKey = "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},13"
# PKEY_SFX_ProcessingModes_Supported_For_Streaming = {d3993a3f-...},5  (processing modes)
$procModesKey = "{d3993a3f-99c2-4402-b5ec-a92a0367664b},5"
$defaultMode = "{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}"

$beforeClsid = (Get-ItemProperty $fxPath -Name $compSfxClsidKey -ErrorAction SilentlyContinue).$compSfxClsidKey
$beforeModes = (Get-ItemProperty $fxPath -Name $procModesKey -ErrorAction SilentlyContinue).$procModesKey
"Composite SFX CLSIDs (,13) before: $($beforeClsid -join ', ')" | Out-File $log -Append -Encoding ascii
"Processing Modes (d3993a3f,5) before: $($beforeModes -join ', ')" | Out-File $log -Append -Encoding ascii

try {
    # Write our CLSID to the composite SFX CLSID list
    Set-ItemProperty $fxPath -Name $compSfxClsidKey -Value @($ourClsid) -Type MultiString
    $afterClsid = (Get-ItemProperty $fxPath -Name $compSfxClsidKey).$compSfxClsidKey
    "Composite SFX CLSIDs (,13) after: $($afterClsid -join ', ')" | Out-File $log -Append -Encoding ascii

    # Ensure processing modes has default mode (not our CLSID!)
    Set-ItemProperty $fxPath -Name $procModesKey -Value @($defaultMode) -Type MultiString
    $afterModes = (Get-ItemProperty $fxPath -Name $procModesKey).$procModesKey
    "Processing Modes (d3993a3f,5) after: $($afterModes -join ', ')" | Out-File $log -Append -Encoding ascii
} catch {
    "FxProperties write FAILED: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
}

# Step 5: Delete DisableEnhancements if present
try {
    Remove-ItemProperty $fxPath -Name "{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5" -ErrorAction SilentlyContinue
} catch {}

# Step 6: Restart audio services
Remove-Item "C:\ProgramData\HrtfApo\debug.log" -ErrorAction SilentlyContinue
net stop audiosrv /y 2>&1 | Out-File $log -Append -Encoding ascii
net stop AudioEndpointBuilder /y 2>&1 | Out-File $log -Append -Encoding ascii
Start-Sleep 1
net start AudioEndpointBuilder 2>&1 | Out-File $log -Append -Encoding ascii
net start audiosrv 2>&1 | Out-File $log -Append -Encoding ascii

# Verify and re-apply if AudioEndpointBuilder cleared our values
Start-Sleep 2

$verifyClsid = (Get-ItemProperty $fxPath -Name $compSfxClsidKey -ErrorAction SilentlyContinue).$compSfxClsidKey
"Composite SFX (,13) after restart: $($verifyClsid -join ', ')" | Out-File $log -Append -Encoding ascii

if (-not $verifyClsid -or $verifyClsid -notcontains $ourClsid) {
    "Re-applying composite SFX CLSID after restart..." | Out-File $log -Append -Encoding ascii
    try {
        Set-ItemProperty $fxPath -Name $compSfxClsidKey -Value @($ourClsid) -Type MultiString
        Set-ItemProperty $fxPath -Name $procModesKey -Value @($defaultMode) -Type MultiString
        # Also ensure legacy SFX is set
        Set-ItemProperty $fxPath -Name "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5" -Value $ourClsid -Type String
        $final = (Get-ItemProperty $fxPath -Name $compSfxClsidKey).$compSfxClsidKey
        "Re-applied. Composite SFX (,13) now: $($final -join ', ')" | Out-File $log -Append -Encoding ascii
    } catch {
        "Re-apply FAILED: $($_.Exception.Message)" | Out-File $log -Append -Encoding ascii
    }
}

# Delete old debug log and play test tone
Remove-Item "C:\ProgramData\HrtfApo\debug.log" -Force -ErrorAction SilentlyContinue
Start-Sleep 1

# Play audio to trigger APO loading
"Playing test tone..." | Out-File $log -Append -Encoding ascii
Start-Process ffplay -ArgumentList '-nodisp','-autoexit','-t','3','-f','lavfi','-i','sine=frequency=440:duration=3' -Wait -ErrorAction SilentlyContinue
Start-Sleep 2

# Check results
if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
    "=== debug.log ===" | Out-File $log -Append -Encoding ascii
    Get-Content "C:\ProgramData\HrtfApo\debug.log" | Out-File $log -Append -Encoding ascii
} else {
    "No debug.log" | Out-File $log -Append -Encoding ascii
}

"Done" | Out-File $log -Append -Encoding ascii
