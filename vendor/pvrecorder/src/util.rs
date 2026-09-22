/*
    Copyright 2021-2025 Picovoice Inc.

    You may not use this file except in compliance with the license. A copy of the license is located in the "LICENSE"
    file accompanying this source.

    Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on
    an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
    specific language governing permissions and limitations under the License.
*/

use std::path::PathBuf;

const DEFAULT_RELATIVE_LIBRARY_DIR: &str = "lib/";

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
fn find_machine_type() -> String {
    use std::process::Command;

    // FIX: Changed from panic to graceful fallback with warning
    let cpu_info = match Command::new("cat").arg("/proc/cpuinfo").output() {
        Ok(output) => output,
        Err(e) => {
            eprintln!("WARNING: Failed to read /proc/cpuinfo: {}. Using fallback.", e);
            return String::from("unsupported");
        }
    };

    let cpu_info_str = match std::str::from_utf8(&cpu_info.stdout) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("WARNING: /proc/cpuinfo contains invalid UTF-8. Using fallback.");
            return String::from("unsupported");
        }
    };

    let cpu_part_list: Vec<&str> = cpu_info_str
        .lines()  // FIX: Use lines() instead of split("\n") for cross-platform compatibility
        .filter(|x| x.contains("CPU part"))
        .collect();

    // FIX: Use is_empty() instead of len() == 0
    if cpu_part_list.is_empty() {
        eprintln!("WARNING: Could not find CPU part in /proc/cpuinfo. Using fallback.");
        return String::from("unsupported");
    }

    let cpu_part = cpu_part_list[0]
        .split_whitespace()  // FIX: More robust than split(" ")
        .last()
        .unwrap_or("unknown")
        .to_lowercase();

    let machine = match cpu_part.as_str() {
        "0xb76" => "arm11",
        "0xd03" => "cortex-a53",
        "0xd08" => "cortex-a72",
        "0xd0b" => "cortex-a76",
        _ => "unsupported",
    };

    String::from(machine)
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn base_library_path() -> PathBuf {
    PathBuf::from("mac/x86_64/libpv_recorder.dylib")
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn base_library_path() -> PathBuf {
    PathBuf::from("mac/arm64/libpv_recorder.dylib")
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn base_library_path() -> PathBuf {
    PathBuf::from("windows/amd64/libpv_recorder.dll")
}

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
fn base_library_path() -> PathBuf {
    PathBuf::from("windows/arm64/libpv_recorder.dll")
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn base_library_path() -> PathBuf {
    PathBuf::from("linux/x86_64/libpv_recorder.so")
}

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
fn base_library_path() -> PathBuf {
    const RPI_MACHINES: [&str; 4] = ["arm11", "cortex-a53", "cortex-a72", "cortex-a76"];

    let machine = find_machine_type();
    match machine.as_str() {
        machine if RPI_MACHINES.contains(&machine) => {
            if cfg!(target_arch = "aarch64") {
                PathBuf::from(format!(
                    "raspberry-pi/{}-aarch64/libpv_recorder.so",
                    machine
                ))
            } else {
                PathBuf::from(format!("raspberry-pi/{}/libpv_recorder.so", machine))
            }
        }
        _ => {
            eprintln!(
                "WARNING: Device not officially supported by Picovoice. \
                Falling back to the armv6-based (Raspberry Pi Zero) library. \
                This is not tested nor optimal. For best results, use Raspberry Pi's models."
            );
            PathBuf::from("raspberry-pi/arm11/libpv_recorder.so")
        }
    }
}

/// Returns the default path to the pvrecorder library for the current platform.
#[must_use]
pub fn pv_library_path() -> PathBuf {
    PathBuf::from(env!("OUT_DIR"))
        .join(DEFAULT_RELATIVE_LIBRARY_DIR)
        .join(base_library_path())
}
