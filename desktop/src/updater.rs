#[cfg(target_os = "windows")]
use crate::i18n::{Locale, Text};
#[cfg(any(target_os = "windows", test))]
use serde::Deserialize;

#[cfg(target_os = "windows")]
const RELEASE_API: &str = "https://api.github.com/repos/andrecrjr/GamepadDesktop/releases/latest";
#[cfg(any(target_os = "windows", test))]
const WINDOWS_ASSET: &str = "GamepadDesktop-Windows-x64.exe";

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug)]
struct UpdateInfo {
    tag_name: String,
    asset: GitHubAsset,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(String),
    Downloading(String),
    UpToDate(String),
    Failed(String),
    Restarting,
}

#[cfg(any(target_os = "windows", test))]
fn parse_sha256_digest(value: &str) -> Option<&str> {
    let digest = value.strip_prefix("sha256:")?;
    (digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(digest)
}

#[cfg(any(target_os = "windows", test))]
fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut digest = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        digest.push(HEX[(byte >> 4) as usize] as char);
        digest.push(HEX[(byte & 0x0f) as usize] as char);
    }
    digest
}

#[cfg(any(target_os = "windows", test))]
fn update_needed(local_digest: &str, release_digest: &str) -> Result<bool, String> {
    let release_digest = parse_sha256_digest(release_digest)
        .ok_or_else(|| "GitHub did not provide a valid SHA-256 digest".to_owned())?;
    Ok(!local_digest.eq_ignore_ascii_case(release_digest))
}

#[cfg(any(target_os = "windows", test))]
fn update_asset(release: GitHubRelease, local_digest: &str) -> Result<Option<UpdateInfo>, String> {
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == WINDOWS_ASSET)
        .ok_or_else(|| "The latest release has no Windows executable".to_owned())?;
    let digest = asset
        .digest
        .as_deref()
        .ok_or_else(|| "GitHub did not provide the executable digest".to_owned())?;

    if update_needed(local_digest, digest)? {
        Ok(Some(UpdateInfo {
            tag_name: release.tag_name,
            asset,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(target_os = "windows")]
pub struct Updater {
    status: UpdateStatus,
    available: Option<UpdateInfo>,
    receiver: Option<std::sync::mpsc::Receiver<WorkerResult>>,
}

#[cfg(target_os = "windows")]
enum WorkerResult {
    Checked(Result<(String, Option<UpdateInfo>), String>),
    Installed(Result<(), String>),
}

#[cfg(target_os = "windows")]
impl Default for Updater {
    fn default() -> Self {
        Self {
            status: UpdateStatus::Idle,
            available: None,
            receiver: None,
        }
    }
}

#[cfg(target_os = "windows")]
impl Updater {
    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }

    pub fn button_text(&self) -> Text {
        match self.status {
            UpdateStatus::Checking => Text::CheckingForUpdates,
            UpdateStatus::Available(_) => Text::InstallUpdateAndRestart,
            UpdateStatus::Downloading(_) => Text::DownloadingUpdate,
            UpdateStatus::Restarting => Text::RestartingUpdate,
            UpdateStatus::Idle | UpdateStatus::UpToDate(_) | UpdateStatus::Failed(_) => {
                Text::CheckForUpdates
            }
        }
    }

    pub fn enabled(&self) -> bool {
        matches!(
            self.status,
            UpdateStatus::Idle
                | UpdateStatus::Available(_)
                | UpdateStatus::UpToDate(_)
                | UpdateStatus::Failed(_)
        )
    }

    pub fn message(&self, locale: Locale) -> Option<String> {
        match &self.status {
            UpdateStatus::Available(tag) => {
                Some(format!("{}{tag}", locale.text(Text::UpdateAvailablePrefix)))
            }
            UpdateStatus::Downloading(tag) => Some(format!(
                "{}{tag}",
                locale.text(Text::DownloadingUpdatePrefix)
            )),
            UpdateStatus::UpToDate(tag) => {
                Some(format!("{}{tag}", locale.text(Text::UpdateUpToDatePrefix)))
            }
            UpdateStatus::Failed(error) => {
                Some(format!("{}{error}", locale.text(Text::UpdateFailedPrefix)))
            }
            UpdateStatus::Restarting => Some(locale.text(Text::RestartingUpdate).to_owned()),
            UpdateStatus::Idle | UpdateStatus::Checking => None,
        }
    }

    pub fn activate(&mut self) {
        match &self.status {
            UpdateStatus::Available(_) => self.start_install(),
            UpdateStatus::Checking | UpdateStatus::Downloading(_) | UpdateStatus::Restarting => {}
            UpdateStatus::Idle | UpdateStatus::UpToDate(_) | UpdateStatus::Failed(_) => {
                self.start_check();
            }
        }
    }

    pub fn poll(&mut self) -> bool {
        let result = match self.receiver.as_ref() {
            Some(receiver) => match receiver.try_recv() {
                Ok(result) => result,
                Err(std::sync::mpsc::TryRecvError::Empty) => return false,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.receiver = None;
                    self.status =
                        UpdateStatus::Failed("The update worker stopped unexpectedly".to_owned());
                    return false;
                }
            },
            None => return false,
        };
        self.receiver = None;

        match result {
            WorkerResult::Checked(Ok((_, Some(update)))) => {
                self.status = UpdateStatus::Available(update.tag_name.clone());
                self.available = Some(update);
            }
            WorkerResult::Checked(Ok((tag, None))) => {
                self.status = UpdateStatus::UpToDate(tag);
                self.available = None;
            }
            WorkerResult::Checked(Err(error)) | WorkerResult::Installed(Err(error)) => {
                self.status = UpdateStatus::Failed(error);
                self.available = None;
            }
            WorkerResult::Installed(Ok(())) => {
                self.status = UpdateStatus::Restarting;
                return true;
            }
        }
        false
    }

    fn start_check(&mut self) {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.status = UpdateStatus::Checking;
        self.available = None;
        self.receiver = Some(receiver);
        std::thread::spawn(move || {
            let _ = sender.send(WorkerResult::Checked(check_for_update()));
        });
    }

    fn start_install(&mut self) {
        let Some(update) = self.available.clone() else {
            self.start_check();
            return;
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        self.status = UpdateStatus::Downloading(update.tag_name.clone());
        self.receiver = Some(receiver);
        std::thread::spawn(move || {
            let _ = sender.send(WorkerResult::Installed(download_and_schedule(&update)));
        });
    }
}

#[cfg(target_os = "windows")]
fn check_for_update() -> Result<(String, Option<UpdateInfo>), String> {
    use reqwest::blocking::Client;
    use sha2::{Digest, Sha256};
    use std::{fs::File, io::Read, time::Duration};

    let mut executable = File::open(std::env::current_exe().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = executable
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let local_digest = hex_digest(hasher.finalize().as_ref());
    let client = Client::builder()
        .user_agent(concat!("Gamepad Desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .map_err(|error| error.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("No published release is available yet".to_owned());
    }
    let release = response
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json::<GitHubRelease>()
        .map_err(|error| error.to_string())?;
    let tag = release.tag_name.clone();
    Ok((tag, update_asset(release, &local_digest)?))
}

#[cfg(target_os = "windows")]
fn download_and_schedule(update: &UpdateInfo) -> Result<(), String> {
    use reqwest::blocking::Client;
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        time::Duration,
    };

    let target = std::env::current_exe().map_err(|error| error.to_string())?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let staged = target.with_file_name(format!(
        ".gamepad-desktop-update-{}-{nonce}.exe",
        std::process::id()
    ));
    let result = (|| {
        let client = Client::builder()
            .user_agent(concat!("Gamepad Desktop/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|error| error.to_string())?;
        let mut response = client
            .get(&update.asset.browser_download_url)
            .send()
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
            .map_err(|error| format!("Cannot write beside the app: {error}"))?;
        std::io::copy(&mut response, &mut file).map_err(|error| error.to_string())?;
        file.flush().map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);

        let expected = update
            .asset
            .digest
            .as_deref()
            .and_then(parse_sha256_digest)
            .ok_or_else(|| "The release has no valid executable digest".to_owned())?;
        let mut staged_file = File::open(&staged).map_err(|error| error.to_string())?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = staged_file
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        let actual = hex_digest(hasher.finalize().as_ref());
        if !actual.eq_ignore_ascii_case(expected) {
            return Err("The downloaded executable failed its SHA-256 check".to_owned());
        }
        spawn_update_helper(&target, &staged)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

#[cfg(target_os = "windows")]
fn spawn_update_helper(target: &std::path::Path, staged: &std::path::Path) -> Result<(), String> {
    use std::{
        fs,
        os::windows::process::CommandExt,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let script_path = std::env::temp_dir().join(format!(
        "gamepad-desktop-update-{}-{nonce}.ps1",
        std::process::id()
    ));
    let script = format!(
        "$ErrorActionPreference = 'Stop'\n$target = {}\n$staged = {}\n$scriptPath = {}\n$backup = \"$target.update-backup\"\n$oldPid = {}\ntry {{\n  while (Get-Process -Id $oldPid -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 200 }}\n  if (Test-Path -LiteralPath $backup) {{ Remove-Item -LiteralPath $backup -Force }}\n  Move-Item -LiteralPath $target -Destination $backup -Force\n  Move-Item -LiteralPath $staged -Destination $target -Force\n  Start-Process -FilePath $target -WorkingDirectory (Split-Path -Parent $target)\n}} catch {{\n  if (Test-Path -LiteralPath $backup) {{\n    if (Test-Path -LiteralPath $target) {{ Remove-Item -LiteralPath $target -Force }}\n    Move-Item -LiteralPath $backup -Destination $target -Force\n  }}\n  try {{ Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('Gamepad Desktop could not finish the update. The previous version was restored.', 'Gamepad Desktop') | Out-Null }} catch {{}}\n  if (Test-Path -LiteralPath $target) {{ Start-Process -FilePath $target -WorkingDirectory (Split-Path -Parent $target) }}\n}}\nif (Test-Path -LiteralPath $backup) {{ Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue }}\nRemove-Item -LiteralPath $scriptPath -Force -ErrorAction SilentlyContinue\n",
        powershell_literal(target),
        powershell_literal(staged),
        powershell_literal(&script_path),
        std::process::id()
    );
    fs::write(&script_path, script).map_err(|error| error.to_string())?;
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .creation_flags(0x08000000)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not start the Windows updater: {error}"))
}

#[cfg(target_os = "windows")]
fn powershell_literal(path: &std::path::Path) -> String {
    format!("'{}'", path.to_string_lossy().replace(char::from(39), "''"))
}

#[cfg(test)]
mod tests {
    use super::{
        GitHubRelease, WINDOWS_ASSET, hex_digest, parse_sha256_digest, update_asset, update_needed,
    };

    #[test]
    fn parses_sha256_digest_from_github_asset_metadata() {
        let digest = format!("sha256:{}", "a".repeat(64));
        assert_eq!(parse_sha256_digest(&digest), Some(&digest[7..]));
    }

    #[test]
    fn rejects_malformed_sha256_digests() {
        assert_eq!(parse_sha256_digest("md5:abcd"), None);
        assert_eq!(parse_sha256_digest("sha256:abcd"), None);
        assert_eq!(
            parse_sha256_digest(&format!("sha256:{}g", "a".repeat(63))),
            None
        );
    }

    #[test]
    fn release_digest_detects_a_different_executable_without_repeating_an_installed_one() {
        let current = "a".repeat(64);
        let update = format!("sha256:{}", "b".repeat(64));
        let same = format!("sha256:{current}");

        assert!(update_needed(&current, &update).unwrap());
        assert!(!update_needed(&current, &same).unwrap());
    }

    #[test]
    fn encodes_sha256_bytes_as_lowercase_hex() {
        assert_eq!(hex_digest(&[0xab, 0x00, 0xff]), "ab00ff");
    }

    #[test]
    fn latest_release_requires_the_direct_windows_executable_and_its_digest() {
        let release: GitHubRelease = serde_json::from_str(&format!(
            r#"{{"tag_name":"desktop-v1.2.3","assets":[{{"name":"{WINDOWS_ASSET}","browser_download_url":"https://example.com/app.exe","digest":"sha256:{}"}}]}}"#,
            "a".repeat(64)
        ))
        .unwrap();

        let update = update_asset(release, &"b".repeat(64)).unwrap().unwrap();
        assert_eq!(update.tag_name, "desktop-v1.2.3");
        assert_eq!(
            update.asset.browser_download_url,
            "https://example.com/app.exe"
        );
    }
}
