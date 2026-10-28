#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::i18n::{Locale, Text};
#[cfg(any(target_os = "windows", target_os = "linux", test))]
use serde::Deserialize;

#[cfg(any(target_os = "windows", target_os = "linux"))]
const RELEASE_API: &str = "https://api.github.com/repos/andrecrjr/GamepadDesktop/releases/latest";
#[cfg(any(target_os = "windows", test))]
const WINDOWS_ASSET: &str = "GamepadDesktop-Windows-x64.exe";
#[cfg(any(target_os = "linux", test))]
const LINUX_ASSET: &str = "GamepadDesktop-Linux-x64.tar.gz";
#[cfg(any(target_os = "linux", test))]
const LINUX_DEB_ASSET: &str = "GamepadDesktop-Linux-x64.deb";
#[cfg(target_os = "linux")]
const LINUX_BUILD_ASSET: &str = "GamepadDesktop-Linux-x64.build";

#[cfg(any(target_os = "windows", target_os = "linux", test))]
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
#[derive(Clone, Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
#[derive(Clone, Debug)]
struct UpdateInfo {
    tag_name: String,
    asset: GitHubAsset,
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
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

#[cfg(any(target_os = "windows", target_os = "linux", test))]
fn parse_sha256_digest(value: &str) -> Option<&str> {
    let digest = value.strip_prefix("sha256:")?;
    (digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(digest)
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
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

#[cfg(target_os = "linux")]
fn update_needed_from_comparison(ahead_by: u32) -> bool {
    ahead_by > 0
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
enum LinuxInstallKind {
    DebPackage,
    Portable,
}

#[cfg(target_os = "linux")]
fn linux_update_asset(
    release: GitHubRelease,
    ahead_by: u32,
    install_kind: LinuxInstallKind,
) -> Result<Option<UpdateInfo>, String> {
    let asset_name = match install_kind {
        LinuxInstallKind::DebPackage => LINUX_DEB_ASSET,
        LinuxInstallKind::Portable => LINUX_ASSET,
    };
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| format!("The latest release has no Linux package ({asset_name})"))?;
    let digest = asset
        .digest
        .as_deref()
        .ok_or_else(|| "GitHub did not provide the Linux archive digest".to_owned())?;
    parse_sha256_digest(digest)
        .ok_or_else(|| "GitHub did not provide a valid Linux archive SHA-256 digest".to_owned())?;

    if update_needed_from_comparison(ahead_by) {
        Ok(Some(UpdateInfo {
            tag_name: release.tag_name,
            asset,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(target_os = "linux")]
fn linux_install_kind() -> LinuxInstallKind {
    let Ok(executable) = std::env::current_exe() else {
        return LinuxInstallKind::Portable;
    };
    let installed_by_dpkg = std::process::Command::new("dpkg-query")
        .args(["--search"])
        .arg(executable)
        .output()
        .is_ok_and(|output| output.status.success());
    if installed_by_dpkg {
        LinuxInstallKind::DebPackage
    } else {
        LinuxInstallKind::Portable
    }
}

#[cfg(target_os = "linux")]
fn linux_build_revision(release: &GitHubRelease, contents: &str) -> Result<String, String> {
    if !release
        .assets
        .iter()
        .any(|asset| asset.name == LINUX_BUILD_ASSET)
    {
        return Err(
            "This Linux release has no build revision metadata; update to a newer release first"
                .to_owned(),
        );
    }
    let revision = contents.trim();
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("The Linux release has an invalid build revision".to_owned());
    }
    Ok(revision.to_owned())
}

pub struct Updater {
    status: UpdateStatus,
    available: Option<UpdateInfo>,
    receiver: Option<std::sync::mpsc::Receiver<WorkerResult>>,
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
enum WorkerResult {
    Checked(Result<(String, Option<UpdateInfo>), String>),
    Installed(Result<(), String>),
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl Default for Updater {
    fn default() -> Self {
        Self {
            status: UpdateStatus::Idle,
            available: None,
            receiver: None,
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
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

#[cfg(target_os = "linux")]
fn check_for_update() -> Result<(String, Option<UpdateInfo>), String> {
    use reqwest::blocking::Client;
    use std::time::Duration;

    let installed_commit = env!("GAMEPAD_DESKTOP_GIT_SHA");
    if installed_commit.len() != 40
        || !installed_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(
            "This build has no Git commit version; rebuild it from a Git checkout".to_owned(),
        );
    }
    let client = Client::builder()
        .user_agent(concat!("Gamepad Desktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| error.to_string())?;
    let release = client
        .get(RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .map_err(|error| error.to_string())?;
    if release.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("No published release is available yet".to_owned());
    }
    let release = release
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json::<GitHubRelease>()
        .map_err(|error| error.to_string())?;
    let tag = release.tag_name.clone();
    let install_kind = linux_install_kind();
    let build_asset = release
        .assets
        .iter()
        .find(|asset| asset.name == LINUX_BUILD_ASSET)
        .ok_or_else(|| {
            "This Linux release has no build revision metadata; update to a newer release first"
                .to_owned()
        })?;
    let build_revision = client
        .get(&build_asset.browser_download_url)
        .send()
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .text()
        .map_err(|error| error.to_string())?;
    let release_commit = linux_build_revision(&release, &build_revision)?;
    let comparison = client
        .get(format!(
            "https://api.github.com/repos/andrecrjr/GamepadDesktop/compare/{installed_commit}...{release_commit}"
        ))
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json::<GitHubComparison>()
        .map_err(|error| error.to_string())?;
    Ok((
        tag,
        linux_update_asset(release, comparison.ahead_by, install_kind)?,
    ))
}

#[cfg(target_os = "linux")]
#[derive(Deserialize)]
struct GitHubComparison {
    ahead_by: u32,
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

#[cfg(target_os = "linux")]
fn download_and_schedule(update: &UpdateInfo) -> Result<(), String> {
    use reqwest::blocking::Client;
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        os::unix::fs::PermissionsExt,
        process::{Command, Stdio},
        time::Duration,
    };

    let target = std::env::current_exe().map_err(|error| error.to_string())?;
    let parent = target
        .parent()
        .ok_or_else(|| "Cannot locate the application directory".to_owned())?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let work = std::env::temp_dir().join(format!(
        "gamepad-desktop-update-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&work).map_err(|error| error.to_string())?;
    let archive = work.join(&update.asset.name);
    let extracted = work.join("gamepad-desktop");
    let staged = parent.join(format!(
        ".gamepad-desktop-update-{}-{nonce}",
        std::process::id()
    ));
    let backup = parent.join(format!(
        ".gamepad-desktop-backup-{}-{nonce}",
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
            .open(&archive)
            .map_err(|error| error.to_string())?;
        std::io::copy(&mut response, &mut file).map_err(|error| error.to_string())?;
        file.flush().map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);

        let expected = update
            .asset
            .digest
            .as_deref()
            .and_then(parse_sha256_digest)
            .ok_or_else(|| "The Linux release has no valid archive digest".to_owned())?;
        let mut archive_file = File::open(&archive).map_err(|error| error.to_string())?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = archive_file
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        let actual = hex_digest(hasher.finalize().as_ref());
        if !actual.eq_ignore_ascii_case(expected) {
            return Err("The downloaded Linux archive failed its SHA-256 check".to_owned());
        }

        if update.asset.name == LINUX_DEB_ASSET {
            install_deb_update(&archive)?;
            spawn_linux_relaunch_helper(&target)?;
            return Ok(());
        }

        let extraction = Command::new("tar")
            .args(["--extract", "--gzip", "--file"])
            .arg(&archive)
            .arg("--directory")
            .arg(&work)
            .args(["--no-same-owner", "--", "gamepad-desktop"])
            .status()
            .map_err(|error| format!("Could not start tar to unpack the update: {error}"))?;
        if !extraction.success() {
            return Err("Could not unpack the Linux update archive".to_owned());
        }
        let metadata = fs::symlink_metadata(&extracted).map_err(|error| error.to_string())?;
        if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err(
                "The Linux archive does not contain an executable gamepad-desktop file".to_owned(),
            );
        }
        fs::copy(&extracted, &staged)
            .map_err(|error| format!("Cannot write beside the app: {error}"))?;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))
            .map_err(|error| error.to_string())?;

        const HELPER: &str = r#"
set -u
pid=$1; target=$2; staged=$3; backup=$4
shift 4
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
if ! mv -- "$target" "$backup"; then exit 1; fi
if ! mv -- "$staged" "$target"; then mv -- "$backup" "$target"; exit 1; fi
"$target" "$@" >/dev/null 2>&1 &
new_pid=$!
sleep 2
if kill -0 "$new_pid" 2>/dev/null; then rm -f -- "$backup"; exit 0; fi
rm -f -- "$target"
mv -- "$backup" "$target"
"$target" "$@" >/dev/null 2>&1 &
"#;
        let mut helper = Command::new("sh");
        helper
            .arg("-c")
            .arg(HELPER)
            .arg("gamepad-desktop-update")
            .arg(std::process::id().to_string())
            .arg(&target)
            .arg(&staged)
            .arg(&backup)
            .args(std::env::args_os().skip(1))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Could not start the Linux update helper: {error}"))?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    let _ = fs::remove_dir_all(&work);
    result
}

#[cfg(target_os = "linux")]
fn install_deb_update(package: &std::path::Path) -> Result<(), String> {
    use std::process::Command;

    let output = Command::new("pkexec")
        .args(["apt", "install", "--yes", "--reinstall"])
        .arg(package)
        .output()
        .map_err(|error| format!("Could not request administrator authorization: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if detail.is_empty() {
        "The Debian package installation was cancelled or failed".to_owned()
    } else {
        format!("The Debian package installation failed: {detail}")
    })
}

#[cfg(target_os = "linux")]
fn spawn_linux_relaunch_helper(target: &std::path::Path) -> Result<(), String> {
    use std::process::{Command, Stdio};

    const HELPER: &str = r#"
pid=$1; target=$2
shift 2
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
"$target" "$@" >/dev/null 2>&1 &
"#;
    Command::new("sh")
        .args(["-c", HELPER, "gamepad-desktop-restart"])
        .arg(std::process::id().to_string())
        .arg(target)
        .args(std::env::args_os().skip(1))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not start the Linux restart helper: {error}"))
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

    #[cfg(target_os = "linux")]
    #[test]
    fn release_is_newer_only_when_the_installed_commit_is_behind() {
        use super::update_needed_from_comparison;

        assert!(update_needed_from_comparison(3));
        assert!(!update_needed_from_comparison(0));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_release_selects_the_asset_for_its_installation_type() {
        use super::{LINUX_ASSET, LINUX_DEB_ASSET, LinuxInstallKind, linux_update_asset};

        let release = || {
            serde_json::from_value::<GitHubRelease>(serde_json::json!({
                "tag_name": "desktop-v1.2.3",
                "assets": [
                    {
                        "name": LINUX_ASSET,
                        "browser_download_url": "https://example.com/app.tar.gz",
                        "digest": format!("sha256:{}", "a".repeat(64))
                    },
                    {
                        "name": LINUX_DEB_ASSET,
                        "browser_download_url": "https://example.com/app.deb",
                        "digest": format!("sha256:{}", "b".repeat(64))
                    }
                ]
            }))
            .unwrap()
        };

        let portable = linux_update_asset(release(), 1, LinuxInstallKind::Portable)
            .unwrap()
            .unwrap();
        assert_eq!(portable.tag_name, "desktop-v1.2.3");
        assert_eq!(
            portable.asset.browser_download_url,
            "https://example.com/app.tar.gz"
        );

        let deb = linux_update_asset(release(), 1, LinuxInstallKind::DebPackage)
            .unwrap()
            .unwrap();
        assert_eq!(
            deb.asset.browser_download_url,
            "https://example.com/app.deb"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_release_requires_a_valid_build_revision() {
        use super::{GitHubRelease, LINUX_BUILD_ASSET, linux_build_revision};

        let release: GitHubRelease = serde_json::from_str(&format!(
            r#"{{"tag_name":"desktop-v1.2.3","assets":[{{"name":"{LINUX_BUILD_ASSET}","browser_download_url":"https://example.com/app.build"}}]}}"#
        ))
        .unwrap();
        assert_eq!(
            linux_build_revision(&release, &format!("{}\n", "a".repeat(40))).unwrap(),
            "a".repeat(40)
        );
        assert!(linux_build_revision(&release, "bad").is_err());
    }

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
