use crate::state::SystemProfile;

#[cfg(target_os = "windows")]
use serde_json::Value;
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

pub(crate) fn detect_system_profile() -> SystemProfile {
    let logical_cores = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(4);

    #[cfg(target_os = "windows")]
    let (total_memory_bytes, gpu_name, gpu_memory_bytes, directml_available) = {
        let mut status = MEMORYSTATUSEX::default();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        let total_memory_bytes = unsafe {
            if GlobalMemoryStatusEx(&mut status).is_ok() {
                status.ullTotalPhys
            } else {
                0
            }
        };
        let (gpu_name, gpu_memory_bytes) = detect_primary_gpu();
        let directml_available = gpu_name.is_some() && directml_runtime_available();
        (
            total_memory_bytes,
            gpu_name,
            gpu_memory_bytes,
            directml_available,
        )
    };

    #[cfg(not(target_os = "windows"))]
    let (total_memory_bytes, gpu_name, gpu_memory_bytes, directml_available) = (0, None, 0, false);

    SystemProfile {
        logical_cores,
        total_memory_bytes,
        gpu_name,
        gpu_memory_bytes,
        directml_available,
    }
}

#[cfg(target_os = "windows")]
fn directml_runtime_available() -> bool {
    crate::runtime::directml_runtime_available()
}

#[cfg(target_os = "windows")]
fn detect_primary_gpu() -> (Option<String>, u64) {
    if let Some(result) = detect_primary_gpu_via_dxgi() {
        return result;
    }

    detect_primary_gpu_via_wmi()
}

#[cfg(target_os = "windows")]
fn detect_primary_gpu_via_dxgi() -> Option<(Option<String>, u64)> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1().ok()? };

    let mut best_name = None;
    let mut best_memory = 0u64;
    let mut index = 0u32;

    loop {
        let adapter: IDXGIAdapter1 = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(_) => break,
        };
        index += 1;

        let description = match unsafe { adapter.GetDesc1() } {
            Ok(description) => description,
            Err(_) => continue,
        };
        if description.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }

        let dedicated_memory = description.DedicatedVideoMemory as u64;
        if dedicated_memory == 0 || dedicated_memory < best_memory {
            continue;
        }

        best_memory = dedicated_memory;
        best_name = Some(wide_string_to_string(&description.Description));
    }

    if best_memory == 0 {
        None
    } else {
        Some((best_name, best_memory))
    }
}

#[cfg(target_os = "windows")]
fn wide_string_to_string(wide: &[u16]) -> String {
    let length = wide
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..length]).trim().to_string()
}

#[cfg(target_os = "windows")]
fn detect_primary_gpu_via_wmi() -> (Option<String>, u64) {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "try { $gpu = Get-CimInstance Win32_VideoController | Sort-Object -Property AdapterRAM -Descending | Select-Object -First 1 Name,AdapterRAM; if ($gpu) { $gpu | ConvertTo-Json -Compress } } catch { '' }",
        ])
        .output();

    let Ok(output) = output else {
        return (None, 0);
    };

    if !output.status.success() {
        return (None, 0);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return (None, 0);
    }

    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return (None, 0);
    };

    let name = value
        .get("Name")
        .and_then(|field| field.as_str())
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .map(ToOwned::to_owned);
    let gpu_memory_bytes = value
        .get("AdapterRAM")
        .and_then(|field| match field {
            Value::Number(value) => value.as_u64(),
            Value::String(value) => value.parse::<u64>().ok(),
            _ => None,
        })
        .unwrap_or(0);

    (name, gpu_memory_bytes)
}
