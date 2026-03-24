//! APO installation and endpoint association.
//!
//! All admin operations shell out to elevated processes via ShellExecuteW("runas")
//! so the GUI itself doesn't need to run as administrator.

use std::path::PathBuf;

use windows::core::GUID;
use windows::Win32::Foundation::*;
use windows::Win32::System::Memory::*;
use windows::Win32::System::Registry::*;

/// Must match CLSID_HrtfApo in hrtf-apo/cpp/guids.h
const CLSID_HRTF_APO: GUID = GUID::from_u128(0xa1b2c3d4_e5f6_7890_abcd_ef0123456789);

/// The APO DLL embedded at compile time.
const EMBEDDED_DLL: &[u8] = include_bytes!(env!("HRTF_APO_DLL_PATH"));

fn guid_to_string(guid: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1, guid.data2, guid.data3,
        guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
        guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7],
    )
}

fn program_data_dir() -> PathBuf {
    let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    PathBuf::from(pd).join("HrtfApo")
}

fn system32_dll_path() -> PathBuf {
    let win = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\WINDOWS".into());
    PathBuf::from(win).join("System32").join("hrtf_apo.dll")
}

// ── Elevated process helpers ─────────────────────────────────────────────

/// Run a command elevated and wait for it to complete.
/// Uses ShellExecuteExW so we get a process handle to wait on.
fn run_elevated_wait(exe: &str, args: &str) -> Result<(), String> {
    use windows::Win32::UI::Shell::*;
    use windows::Win32::System::Threading::*;
    use windows::core::PCWSTR;

    let exe_w: Vec<u16> = exe.encode_utf16().chain(std::iter::once(0)).collect();
    let args_w: Vec<u16> = args.encode_utf16().chain(std::iter::once(0)).collect();
    let verb_w: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();

    let mut sei = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        hwnd: HWND::default(),
        lpVerb: PCWSTR(verb_w.as_ptr()),
        lpFile: PCWSTR(exe_w.as_ptr()),
        lpParameters: PCWSTR(args_w.as_ptr()),
        lpDirectory: PCWSTR::null(),
        nShow: windows::Win32::UI::WindowsAndMessaging::SW_HIDE.0 as i32,
        hInstApp: windows::Win32::Foundation::HINSTANCE::default(),
        lpIDList: std::ptr::null_mut(),
        lpClass: PCWSTR::null(),
        hkeyClass: HKEY::default(),
        dwHotKey: 0,
        Anonymous: Default::default(),
        hProcess: HANDLE::default(),
    };

    unsafe { ShellExecuteExW(&mut sei) }
        .map_err(|e| format!("UAC denied or executable not found: {e}"))?;

    if !sei.hProcess.is_invalid() {
        unsafe {
            WaitForSingleObject(sei.hProcess, 60000); // 60s timeout
            let mut exit_code: u32 = 0;
            let _ = GetExitCodeProcess(sei.hProcess, &mut exit_code);
            CloseHandle(sei.hProcess).ok();
            if exit_code != 0 {
                return Err(format!("Process exited with code {exit_code}"));
            }
        }
    }

    Ok(())
}

// ── Public API ───────────────────────────────────────────────────────────

/// Check if the APO is registered in the Windows registry.
pub fn is_apo_installed() -> bool {
    let clsid_str = guid_to_string(&CLSID_HRTF_APO);
    let key = format!(r"SOFTWARE\Classes\CLSID\{clsid_str}\InProcServer32");
    reg_get_string(HKEY_LOCAL_MACHINE, &key, "").is_ok()
}

/// Check if the installed DLL differs from the embedded one.
pub fn dll_needs_update() -> bool {
    if EMBEDDED_DLL.is_empty() {
        return false;
    }
    let dll_path = system32_dll_path();
    match std::fs::read(&dll_path) {
        Ok(installed) => installed != EMBEDDED_DLL,
        Err(_) => true,
    }
}

/// Check if the IR file exists in ProgramData.
pub fn ir_file_exists() -> bool {
    program_data_dir().join("hrtf_irs.bin").exists()
}

/// Install the APO (single UAC prompt):
/// 1. Extract DLL to ProgramData (non-elevated)
/// 2. Elevated: copy to System32, regsvr32, set DisableProtectedAudioDG
pub fn register_apo() -> Result<(), String> {
    if EMBEDDED_DLL.is_empty() {
        return Err("APO DLL not embedded. Build hrtf-apo first, then rebuild GUI.".into());
    }

    // 1. Write DLL to ProgramData (non-elevated — any user can write here)
    let install_dir = program_data_dir();
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Failed to create {}: {e}", install_dir.display()))?;

    let pd_dll = install_dir.join("hrtf_apo.dll");
    std::fs::write(&pd_dll, EMBEDDED_DLL)
        .map_err(|e| format!("Failed to write DLL to ProgramData: {e}"))?;

    // 2. Single elevated cmd: stop audio services (unlock DLL) + copy + regsvr32 + reg add + restart
    let sys32_dll = system32_dll_path();
    let batch = format!(
        r#"/c net stop audiosrv /y & net stop AudioEndpointBuilder /y & timeout /t 1 /nobreak >nul & copy /Y "{pd}" "{s32}" && regsvr32 /s "{s32}" && reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio" /v DisableProtectedAudioDG /t REG_DWORD /d 1 /f && net start AudioEndpointBuilder && net start audiosrv"#,
        pd = pd_dll.display(),
        s32 = sys32_dll.display(),
    );
    run_elevated_wait("cmd.exe", &batch)
        .map_err(|e| format!("Install failed: {e}"))?;

    Ok(())
}

/// List active audio render endpoints. Returns vec of (index, endpoint_guid, friendly_name).
pub fn list_audio_endpoints() -> Result<Vec<(u32, String, String)>, String> {
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("Failed to create device enumerator: {e}"))?;

        let collection = enumerator
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .map_err(|e| format!("Failed to enumerate endpoints: {e}"))?;

        let count = collection
            .GetCount()
            .map_err(|e| format!("GetCount failed: {e}"))?;

        let mut result = Vec::new();
        for i in 0..count {
            let device = collection.Item(i).map_err(|e| format!("Item failed: {e}"))?;

            // Get device ID — looks like: {0.0.0.00000000}.{GUID}
            let id = device.GetId().map_err(|e| format!("GetId failed: {e}"))?;
            let id_str = id.to_string().map_err(|_| "Failed to read device ID".to_string())?;
            let endpoint_guid = id_str
                .rsplit('.')
                .next()
                .unwrap_or(&id_str)
                .to_string();

            // Get friendly name
            let props = device
                .OpenPropertyStore(STGM(0))
                .map_err(|e| format!("OpenPropertyStore failed: {e}"))?;
            let name_pv = props
                .GetValue(&PKEY_Device_FriendlyName)
                .map_err(|e| format!("GetValue failed: {e}"))?;
            let name = format!("{}", name_pv).trim_matches('"').to_string();

            result.push((i, endpoint_guid, name));
        }

        Ok(result)
    }
}

/// Associate the APO with a specific endpoint by GUID (single UAC prompt).
/// Runs InstallForEndpoint, restarts audio services, re-applies SFX in one elevated cmd.
pub fn associate_endpoint(endpoint_guid: &str) -> Result<(), String> {
    let sys32_dll = system32_dll_path();
    if !sys32_dll.exists() {
        return Err("APO DLL not found in System32. Run Install first.".into());
    }

    // Single elevated cmd: InstallForEndpoint + restart services + re-apply
    let batch = format!(
        r#"/c rundll32 "{dll}",InstallForEndpoint {guid} && net stop audiosrv /y && net stop AudioEndpointBuilder /y && timeout /t 1 /nobreak >nul && net start AudioEndpointBuilder && net start audiosrv && timeout /t 2 /nobreak >nul && rundll32 "{dll}",InstallForEndpoint {guid}"#,
        dll = sys32_dll.display(),
        guid = endpoint_guid,
    );
    run_elevated_wait("cmd.exe", &batch)
        .map_err(|e| format!("Associate failed: {e}"))?;

    Ok(())
}

/// Uninstall the APO (single UAC prompt):
/// Unregisters COM, restores original SFX, deletes DLL, restarts services.
pub fn uninstall_apo() -> Result<(), String> {
    let sys32_dll = system32_dll_path();
    let pd = program_data_dir();

    // Build the elevated batch command
    let mut cmds: Vec<String> = Vec::new();

    // Unregister
    if sys32_dll.exists() {
        cmds.push(format!(r#"regsvr32 /u /s "{}""#, sys32_dll.display()));
    }

    // Restore original SFX CLSID if saved
    let original_sfx_path = pd.join("original_sfx.txt");
    if original_sfx_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&original_sfx_path) {
            let mut lines = contents.lines();
            if let (Some(endpoint_guid), Some(original_clsid)) = (lines.next(), lines.next()) {
                cmds.push(format!(
                    r#"reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{}\FxProperties" /v "{{d04e05a6-594b-4fb6-a80d-01af5eed7d1d}},5" /t REG_SZ /d "{}" /f"#,
                    endpoint_guid.trim(),
                    original_clsid.trim()
                ));
            }
        }
    }

    // Delete DLL from System32
    if sys32_dll.exists() {
        cmds.push(format!(r#"del /F "{}""#, sys32_dll.display()));
    }

    // Remove DisableProtectedAudioDG
    cmds.push(r#"reg delete "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio" /v DisableProtectedAudioDG /f"#.into());

    // Restart audio services
    cmds.push("net stop audiosrv /y & net stop AudioEndpointBuilder /y & timeout /t 1 /nobreak >nul & net start AudioEndpointBuilder & net start audiosrv".into());

    let batch = format!("/c {}", cmds.join(" & "));
    run_elevated_wait("cmd.exe", &batch)
        .map_err(|e| format!("Uninstall failed: {e}"))?;

    // Delete ProgramData files (non-elevated)
    let _ = std::fs::remove_file(pd.join("hrtf_apo.dll"));
    let _ = std::fs::remove_file(pd.join("original_sfx.txt"));

    Ok(())
}

// ── Registry read helper (non-elevated, read-only) ───────────────────────

fn reg_get_string(root: HKEY, subkey: &str, name: &str) -> Result<String, String> {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

        let result = RegOpenKeyExW(
            root,
            windows::core::PCWSTR(subkey_w.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );
        if result.is_err() {
            return Err("Key not found".into());
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut data_type = REG_VALUE_TYPE::default();
        let mut size: u32 = 0;

        let _ = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut data_type),
            None,
            Some(&mut size),
        );

        let mut buffer = vec![0u8; size as usize];
        let result = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut data_type),
            Some(buffer.as_mut_ptr()),
            Some(&mut size),
        );
        let _ = RegCloseKey(hkey);

        if result.is_err() {
            return Err("Value not found".into());
        }

        let wide: &[u16] =
            std::slice::from_raw_parts(buffer.as_ptr() as *const u16, size as usize / 2);
        let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        Ok(String::from_utf16_lossy(&wide[..len]))
    }
}

// ── Bypass flag (shared memory with the APO) ─────────────────────────────

const SHARED_MEM_NAME: &str = "Global\\HrtfApoBypass";

/// Handle to the shared bypass flag. GUI side.
pub struct BypassControl {
    _handle: HANDLE,
    ptr: *mut u8,
}

unsafe impl Send for BypassControl {}
unsafe impl Sync for BypassControl {}

impl BypassControl {
    /// Open or create the shared memory for the bypass flag.
    pub fn open() -> Option<Self> {
        unsafe {
            let name_w: Vec<u16> = SHARED_MEM_NAME
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                4,
                windows::core::PCWSTR(name_w.as_ptr()),
            )
            .ok()?;

            let ptr = MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, 4);
            if ptr.Value.is_null() {
                let _ = CloseHandle(handle);
                return None;
            }

            Some(Self {
                _handle: handle,
                ptr: ptr.Value as *mut u8,
            })
        }
    }

    pub fn is_bypassed(&self) -> bool {
        unsafe { std::ptr::read_volatile(self.ptr) != 0 }
    }

    pub fn set_bypassed(&self, bypassed: bool) {
        unsafe { std::ptr::write_volatile(self.ptr, if bypassed { 1 } else { 0 }) }
    }
}

impl Drop for BypassControl {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.ptr as *mut _,
            });
            let _ = CloseHandle(self._handle);
        }
    }
}
