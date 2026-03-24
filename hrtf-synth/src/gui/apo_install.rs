//! APO installation and endpoint association — runs directly in the GUI process.
//!
//! Handles COM/APO registry registration and endpoint property store writes
//! so the user doesn't need a separate installer binary.

use windows::core::GUID;
use windows::Win32::System::Registry::*;

/// Must match CLSID_HRTF_APO in hrtf-apo/src/lib.rs
const CLSID_HRTF_APO: GUID = GUID::from_u128(0xa1b2c3d4_e5f6_7890_abcd_ef0123456789);

/// The APO DLL embedded at compile time. Built by: cargo build --release -p hrtf-apo
const EMBEDDED_DLL: &[u8] = include_bytes!(env!("HRTF_APO_DLL_PATH"));
const APO_FRIENDLY_NAME: &str = "Personalized HRTF Binaural";

/// PKEY_FX_StreamEffectClsid = {D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}, 5
const PKEY_FX_STREAM_EFFECT_CLSID: windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY =
    windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY {
        fmtid: GUID::from_u128(0xD04E05A6_594B_4FB6_A80D_01AF5EED7D1D),
        pid: 5,
    };

fn guid_to_string(guid: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1, guid.data2, guid.data3,
        guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
        guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7],
    )
}

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
    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    let dll_path = std::path::Path::new(&program_data).join("HrtfApo").join("hrtf_apo.dll");
    match std::fs::read(&dll_path) {
        Ok(installed) => installed != EMBEDDED_DLL,
        Err(_) => true, // DLL missing — needs install
    }
}

/// Check if the IR file exists in ProgramData.
pub fn ir_file_exists() -> bool {
    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    std::path::Path::new(&program_data)
        .join("HrtfApo")
        .join("hrtf_irs.bin")
        .exists()
}

/// Install the APO: extract embedded DLL to ProgramData, register COM class and APO.
pub fn register_apo() -> Result<(), String> {
    if EMBEDDED_DLL.is_empty() {
        return Err("APO DLL not embedded. Build hrtf-apo first, then rebuild GUI.".into());
    }

    // 1. Extract DLL to ProgramData\HrtfApo\hrtf_apo.dll
    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    let install_dir = std::path::Path::new(&program_data).join("HrtfApo");
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Failed to create install directory: {e}"))?;

    let dll_path = install_dir.join("hrtf_apo.dll");
    std::fs::write(&dll_path, EMBEDDED_DLL)
        .map_err(|e| format!("Failed to write DLL: {e}"))?;

    let dll_str = dll_path.to_string_lossy();
    let clsid_str = guid_to_string(&CLSID_HRTF_APO);

    // 2. COM class registration
    let com_key = format!(r"SOFTWARE\Classes\CLSID\{clsid_str}\InProcServer32");
    reg_set_string(HKEY_LOCAL_MACHINE, &com_key, "", &dll_str)?;
    reg_set_string(HKEY_LOCAL_MACHINE, &com_key, "ThreadingModel", "Both")?;

    // 3. APO registration
    let apo_key = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\{clsid_str}"
    );
    reg_set_string(HKEY_LOCAL_MACHINE, &apo_key, "FriendlyName", APO_FRIENDLY_NAME)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key, "MajorVersion", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key, "MinorVersion", 0)?;
    // FRAMESPERSECOND_MUST_MATCH | BITSPERSAMPLE_MUST_MATCH = 0x0C
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key, "Flags", 0x0000000C)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key, "MinInputConnections", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key, "MaxInputConnections", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key, "MinOutputConnections", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key, "MaxOutputConnections", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &apo_key, "MaxInstances", 0xFFFFFFFF)?;

    Ok(())
}

/// List active audio render endpoints. Returns vec of (index, friendly_name).
pub fn list_audio_endpoints() -> Result<Vec<(u32, String)>, String> {
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|e| format!("Failed to create device enumerator: {e}"))?;

        let collection = enumerator
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .map_err(|e| format!("Failed to enumerate endpoints: {e}"))?;

        let count = collection.GetCount().map_err(|e| format!("GetCount failed: {e}"))?;

        let mut result = Vec::new();
        for i in 0..count {
            let device = collection.Item(i).map_err(|e| format!("Item failed: {e}"))?;
            let props = device
                .OpenPropertyStore(STGM(0))
                .map_err(|e| format!("OpenPropertyStore failed: {e}"))?;
            let name_pv = props
                .GetValue(&PKEY_Device_FriendlyName)
                .map_err(|e| format!("GetValue failed: {e}"))?;
            let name = format!("{}", name_pv).trim_matches('"').to_string();
            result.push((i, name));
        }

        Ok(result)
    }
}

/// Associate the APO with the audio endpoint at the given index.
/// Writes the SFX CLSID directly to the endpoint's FxProperties registry key.
pub fn associate_endpoint(index: u32) -> Result<String, String> {
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|e| format!("Failed to create device enumerator: {e}"))?;

        let collection = enumerator
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .map_err(|e| format!("Failed to enumerate endpoints: {e}"))?;

        let device = collection.Item(index).map_err(|e| format!("Item failed: {e}"))?;

        // Get device ID to find its registry path
        let id = device.GetId().map_err(|e| format!("GetId failed: {e}"))?;
        let id_str = id.to_string().map_err(|_| "Failed to read device ID".to_string())?;

        // Get friendly name
        let props = device.OpenPropertyStore(STGM(0)).map_err(|e| format!("{e}"))?;
        let name_pv = props
            .GetValue(&PKEY_Device_FriendlyName)
            .map_err(|e| format!("{e}"))?;
        let name = format!("{}", name_pv).trim_matches('"').to_string();

        // Find the endpoint GUID in the registry — search active render endpoints
        // The device ID looks like: {0.0.0.00000000}.{GUID}
        // The registry key is: HKLM\...\MMDevices\Audio\Render\{GUID}\FxProperties
        let endpoint_guid = id_str.rsplit('.').next()
            .ok_or("Could not parse endpoint GUID from device ID")?;

        let fx_key = format!(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\{endpoint_guid}\FxProperties"
        );

        let clsid_str = guid_to_string(&CLSID_HRTF_APO);

        // Legacy: PKEY_FX_StreamEffectClsid = {d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5 (REG_SZ)
        reg_set_string(
            HKEY_LOCAL_MACHINE, &fx_key,
            "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5",
            &clsid_str,
        )?;

        // Windows 10 1903+: PKEY_CompositeFX_StreamEffectClsid = {d3993a3f-99c2-4402-b5ec-a92a0367664b},5
        // Must be REG_MULTI_SZ (array of GUID strings)
        reg_set_multi_string(
            HKEY_LOCAL_MACHINE, &fx_key,
            "{d3993a3f-99c2-4402-b5ec-a92a0367664b},5",
            &[&clsid_str],
        )?;

        Ok(name)
    }
}

// ── Registry helpers ────────────────────────────────────────────────────

fn reg_set_string(root: HKEY, subkey: &str, name: &str, value: &str) -> Result<(), String> {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

        let result = RegCreateKeyW(root, windows::core::PCWSTR(subkey_w.as_ptr()), &mut hkey);
        if result.is_err() {
            return Err(format!("Failed to create registry key: {subkey} (run as admin)"));
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let value_w: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
        let value_bytes: &[u8] =
            std::slice::from_raw_parts(value_w.as_ptr() as *const u8, value_w.len() * 2);

        let result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            0,
            REG_SZ,
            Some(value_bytes),
        );
        let _ = RegCloseKey(hkey);

        if result.is_err() {
            return Err(format!("Failed to set registry value: {name}"));
        }
    }
    Ok(())
}

fn reg_set_multi_string(root: HKEY, subkey: &str, name: &str, values: &[&str]) -> Result<(), String> {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

        let result = RegCreateKeyW(root, windows::core::PCWSTR(subkey_w.as_ptr()), &mut hkey);
        if result.is_err() {
            return Err(format!("Failed to create registry key: {subkey} (run as admin)"));
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

        // REG_MULTI_SZ: each string null-terminated, then double-null at the end
        let mut data: Vec<u16> = Vec::new();
        for v in values {
            data.extend(v.encode_utf16());
            data.push(0); // null terminator for this string
        }
        data.push(0); // final double-null

        let data_bytes: &[u8] =
            std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2);

        let result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            0,
            REG_MULTI_SZ,
            Some(data_bytes),
        );
        let _ = RegCloseKey(hkey);

        if result.is_err() {
            return Err(format!("Failed to set registry value: {name}"));
        }
    }
    Ok(())
}

fn reg_set_dword(root: HKEY, subkey: &str, name: &str, value: u32) -> Result<(), String> {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

        let result = RegCreateKeyW(root, windows::core::PCWSTR(subkey_w.as_ptr()), &mut hkey);
        if result.is_err() {
            return Err(format!("Failed to create registry key: {subkey}"));
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let value_bytes = value.to_le_bytes();

        let result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(name_w.as_ptr()),
            0,
            REG_DWORD,
            Some(&value_bytes),
        );
        let _ = RegCloseKey(hkey);

        if result.is_err() {
            return Err(format!("Failed to set registry value: {name}"));
        }
    }
    Ok(())
}

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

// ── Bypass flag (shared memory with the APO) ────────────────────────────

use windows::Win32::Foundation::*;
use windows::Win32::System::Memory::*;

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
            let name_w: Vec<u16> = SHARED_MEM_NAME.encode_utf16().chain(std::iter::once(0)).collect();

            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                4,
                windows::core::PCWSTR(name_w.as_ptr()),
            ).ok()?;

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
            let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: self.ptr as *mut _ });
            let _ = CloseHandle(self._handle);
        }
    }
}
