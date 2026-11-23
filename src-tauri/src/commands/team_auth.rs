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
        macos_session::read(&app, &profile)
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
        macos_session::write(&app, &profile, value.as_deref())
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
    use tauri::AppHandle;

    // Security.framework errSecItemNotFound makes an absent session a normal state.
    const ITEM_NOT_FOUND: i32 = -25_300;

    fn options(app: &AppHandle, profile: &str) -> Result<PasswordOptions, String> {
        validate_profile(profile)?;
        let service = format!("{}.team-session", app.config().identifier);
        let mut options = PasswordOptions::new_generic_password(&service, profile);
        options.set_access_synchronized(Some(false));
        Ok(options)
    }

    pub(super) fn read(app: &AppHandle, profile: &str) -> Result<Option<String>, String> {
        match generic_password(options(app, profile)?) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| "Kayıtlı ekip oturumu çözülemedi.".into()),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
            Err(_) => Err("Kayıtlı ekip oturumu Anahtar Zinciri'nden okunamadı.".into()),
        }
    }

    pub(super) fn write(app: &AppHandle, profile: &str, value: Option<&str>) -> Result<bool, String> {
        let options = options(app, profile)?;
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
