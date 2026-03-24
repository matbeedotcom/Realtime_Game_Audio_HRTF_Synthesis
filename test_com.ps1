try {
    $obj = [System.Runtime.InteropServices.Marshal]::BindToMoniker("new:{A1B2C3D4-E5F6-7890-ABCD-EF0123456789}")
    Write-Host "COM object created successfully: $($obj.GetType())"
} catch {
    Write-Host "COM FAILED: $($_.Exception.Message)"
}

# Also check if debug.log was created by the COM instantiation
if (Test-Path "C:\ProgramData\HrtfApo\debug.log") {
    Write-Host "=== debug.log ==="
    Get-Content "C:\ProgramData\HrtfApo\debug.log"
}
