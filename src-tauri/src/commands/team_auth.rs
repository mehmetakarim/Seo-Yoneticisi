use super::AppState;
#[cfg(windows)]
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn read_team_session(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: String,
) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let _ = app;
        let file = session_path(&state, &profile)?;
        let bytes = match std::fs::read(file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("Kayıtlı ekip oturumu okunamadı.".into()),
        };
        let decrypted = protect(&bytes, false)?;
        return String::from_utf8(decrypted)
            .map(Some)
            .map_err(|_| "Kayıtlı ekip oturumu çözülemedi.".into());
    }
    #[cfg(target_os = "macos")]
    {
        let _ = state;
        macos_session::read(&app.config().identifier, &profile)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (app, state, profile);
        Ok(None)
    }
}

#[tauri::command]
pub fn write_team_session(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: String,
    value: Option<String>,
) -> Result<bool, String> {
    #[cfg(windows)]
    {
        let _ = app;
        let file = session_path(&state, &profile)?;
        if let Some(value) = value {
            validate_session(&value)?;
            let encrypted = protect(value.as_bytes(), true)?;
            let pending = file.with_extension(format!("{}.tmp", std::process::id()));
            std::fs::write(&pending, encrypted)
                .and_then(|()| std::fs::rename(&pending, &file))
                .map_err(|_| "Ekip oturumu bu cihazda saklanamadı.".to_string())?;
        } else if let Err(error) = std::fs::remove_file(file) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err("Kayıtlı ekip oturumu silinemedi.".into());
            }
        }
        return Ok(true);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = state;
        macos_session::write(&app.config().identifier, &profile, value.as_deref())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (app, state, profile, value);
        Ok(false)
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn validate_profile(profile: &str) -> Result<(), String> {
    if profile.len() != 36 || !profile.bytes().all(|byte| byte.is_ascii_hexdigit() || byte == b'-') {
        return Err("Geçersiz ekip oturum profili.".into());
    }
    Ok(())
}

#[cfg(any(windows, target_os = "macos"))]
fn validate_session(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 128 * 1024 {
        return Err("Ekip oturumu kaydedilemedi.".into());
    }
    Ok(())
}

#[cfg(windows)]
fn session_path(state: &AppState, profile: &str) -> Result<PathBuf, String> {
    validate_profile(profile)?;
    Ok(state.db_path.with_file_name(format!("team-session-{profile}.bin")))
}

#[cfg(target_os = "macos")]
mod macos_session {
    use super::{validate_profile, validate_session};
    use security_framework::passwords::{
        delete_generic_password_options, generic_password, set_generic_password_options,
        PasswordOptions,
    };
    // Security.framework errSecItemNotFound makes an absent session a normal state.
    const ITEM_NOT_FOUND: i32 = -25_300;

    fn options(identifier: &str, profile: &str) -> Result<PasswordOptions, String> {
        validate_profile(profile)?;
        let service = format!("{identifier}.team-session");
        let mut options = PasswordOptions::new_generic_password(&service, profile);
        options.set_access_synchronized(Some(false));
        Ok(options)
    }

    pub(super) fn read(identifier: &str, profile: &str) -> Result<Option<String>, String> {
        match generic_password(options(identifier, profile)?) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| "Kayıtlı ekip oturumu çözülemedi.".into()),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
            Err(_) => Err("Kayıtlı ekip oturumu Anahtar Zinciri'nden okunamadı.".into()),
        }
    }

    pub(super) fn write(identifier: &str, profile: &str, value: Option<&str>) -> Result<bool, String> {
        let options = options(identifier, profile)?;
        if let Some(value) = value {
            validate_session(value)?;
            set_generic_password_options(value.as_bytes(), options)
                .map_err(|_| "Ekip oturumu Anahtar Zinciri'ne kaydedilemedi.".to_string())?;
        } else {
            match delete_generic_password_options(options) {
                Ok(()) => {}
                Err(error) if error.code() == ITEM_NOT_FOUND => {}
                Err(_) => return Err("Kayıtlı ekip oturumu Anahtar Zinciri'nden silinemedi.".into()),
            }
        }
        Ok(true)
    }

    #[cfg(test)]
    mod tests {
        use super::{read, validate_profile, write};
        use std::{
            env,
            process::{self, Command},
            time::{SystemTime, UNIX_EPOCH},
        };

        const NAMESPACE: &str = "com.kurumsalit.seo-yoneticisi.keychain-test";
        const PHASE: &str = "SEO_MACOS_KEYCHAIN_TEST_PHASE";
        const PROFILE: &str = "SEO_MACOS_KEYCHAIN_TEST_PROFILE";
        const INITIAL: &str = r#"{"refreshToken":"synthetic-initial","workspace":"Test"}"#;
        const UPDATED: &str = r#"{"refreshToken":"synthetic-updated","workspace":"Deneme alanı"}"#;

        struct Entry {
            identifier: String,
            profile: String,
        }

        impl Entry {
            fn new(profile: String) -> Self {
                validate_profile(&profile).unwrap();
                Self { identifier: format!("{NAMESPACE}.{profile}"), profile }
            }

            fn run(&self, phase: &str) {
                let module = module_path!().split_once("::").unwrap().1;
                let name = format!("{module}::macos_team_session_keychain_round_trip");
                let output = Command::new(env::current_exe().unwrap())
                    .args(["--ignored", "--exact", "--nocapture", &name])
                    .env(PHASE, phase)
                    .env(PROFILE, &self.profile)
                    .output()
                    .unwrap_or_else(|error| panic!("Mac Keychain test process failed to start: {error}"));
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(
                    output.status.success() && stdout.contains(&format!("Keychain phase {phase} complete")),
                    "Mac Keychain phase {phase} failed:\n{stdout}\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        struct Cleanup(Entry);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = write(&self.0.identifier, &self.0.profile, None);
            }
        }

        #[test]
        #[ignore = "requires an unlocked isolated macOS Keychain"]
        fn macos_team_session_keychain_round_trip() {
            if let Ok(phase) = env::var(PHASE) {
                let entry = Entry::new(env::var(PROFILE).unwrap());
                let identifier = &entry.identifier;
                let profile = &entry.profile;
                match phase.as_str() {
                    "missing" => assert_eq!(read(identifier, profile).unwrap(), None),
                    "write" => assert!(write(identifier, profile, Some(INITIAL)).unwrap()),
                    "read" => assert_eq!(read(identifier, profile).unwrap().as_deref(), Some(INITIAL)),
                    "update" => assert!(write(identifier, profile, Some(UPDATED)).unwrap()),
                    "read_updated" => assert_eq!(read(identifier, profile).unwrap().as_deref(), Some(UPDATED)),
                    "delete" => assert!(write(identifier, profile, None).unwrap()),
                    _ => panic!("Unexpected Mac Keychain test phase"),
                }
                println!("Keychain phase {phase} complete");
                return;
            }

            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let hex = format!("{:032x}", (nanos << 32) | u128::from(process::id()));
            let profile = format!(
                "{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..]
            );
            let entry = Cleanup(Entry::new(profile));
            // Each operation runs in a fresh process, just as it does after a desktop restart.
            for phase in ["missing", "write", "read", "update", "read_updated", "delete", "missing", "delete"] {
                entry.0.run(phase);
            }
        }
    }
}

#[cfg(windows)]
fn protect(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{LocalFree, HLOCAL},
            Security::Cryptography::{
                CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
            },
        },
    };
    if bytes.is_empty() || bytes.len() > 256 * 1024 {
        return Err("Ekip oturumu verisi geçersiz.".into());
    }
    let input = CRYPT_INTEGER_BLOB { cbData: bytes.len() as u32, pbData: bytes.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // DPAPI reads the input for this call and allocates the output with LocalAlloc.
    unsafe {
        let result = if encrypt {
            CryptProtectData(&input, PCWSTR::null(), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        } else {
            CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        };
        result.map_err(|_| "Windows ekip oturumunu şifreleyemedi veya açamadı.".to_string())?;
        let copied = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Ok(copied)
    }
}

#[cfg(all(windows, test))]
#[test]
fn windows_team_session_encryption_round_trip() {
    let source = b"synthetic team session";
    let encrypted = protect(source, true).unwrap();
    assert_ne!(encrypted, source);
    assert_eq!(protect(&encrypted, false).unwrap(), source);
}
