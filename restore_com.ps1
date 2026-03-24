$ErrorActionPreference = "Stop"
$log = "C:\ProgramData\HrtfApo\restore.log"

"=== Restore COM $(Get-Date) ===" | Out-File $log -Encoding ascii

# Stop audio
net stop audiosrv 2>&1 | Out-File $log -Append -Encoding ascii
net stop AudioEndpointBuilder 2>&1 | Out-File $log -Append -Encoding ascii
Start-Sleep 1

# Restore COM via reg.exe (runs as current user = admin)
$result = reg add "HKLM\SOFTWARE\Classes\CLSID\{C9453E73-8C5C-4463-9984-AF8BAB2F5447}\InProcServer32" /ve /t REG_SZ /d "C:\WINDOWS\System32\WMALFXGFXDSP.dll" /f 2>&1
"reg result: $result" | Out-File $log -Append -Encoding ascii

# Also restore the 637C490D CLSID if it was hijacked
$result2 = reg add "HKLM\SOFTWARE\Classes\CLSID\{637C490D-EEE3-4C0A-973F-371958802DA2}\InProcServer32" /ve /t REG_SZ /d "C:\WINDOWS\System32\WMALFXGFXDSP.dll" /f 2>&1
"reg 637C result: $result2" | Out-File $log -Append -Encoding ascii

# Verify
$val = (Get-ItemProperty "HKLM:\SOFTWARE\Classes\CLSID\{C9453E73-8C5C-4463-9984-AF8BAB2F5447}\InProcServer32")."(Default)"
"After: $val" | Out-File $log -Append -Encoding ascii

# Restart audio
net start AudioEndpointBuilder 2>&1 | Out-File $log -Append -Encoding ascii
net start audiosrv 2>&1 | Out-File $log -Append -Encoding ascii

"Done" | Out-File $log -Append -Encoding ascii
