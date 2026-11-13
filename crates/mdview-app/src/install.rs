//! Installing downloaded updates and restarting the application.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// What kind of installer a downloaded asset uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallKind {
    /// Replace the running AppImage in place.
    AppImage,
    /// Debian package through `pkexec dpkg`.
    Debian,
    /// RPM package through `pkexec rpm`.
    Rpm,
    /// Flatpak update through the Flatpak daemon.
    Flatpak,
    /// Windows MSI.
    Msi,
    /// macOS disk image opened in the installer.
    Dmg,
    /// No automatic path; the user installs manually.
    Unsupported,
}

/// A resolved installation command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallPlan {
    /// Installer family.
    pub kind: InstallKind,
    /// Program to run, when not an AppImage replacement.
    pub program: Option<String>,
    /// Arguments for the program.
    pub args: Vec<String>,
    /// Whether the application can restart after installation.
    pub restart: bool,
    /// Short explanation shown in the dialog.
    pub description: String,
}

/// Result of an installation attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallOutcome {
    /// Whether a restart is the next step.
    pub restart: bool,
    /// User-facing result text.
    pub message: String,
}

/// Inspect an asset and choose an installation path.
pub fn plan_for(path: &Path, flatpak: bool) -> InstallPlan {
    if flatpak {
        return InstallPlan {
            kind: InstallKind::Flatpak,
            program: Some("flatpak".to_owned()),
            args: vec!["update".to_owned(), "-y".to_owned()],
            restart: true,
            description: "Update the Flatpak installation".to_owned(),
        };
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let display = path.display().to_string();
    match extension.as_str() {
        "appimage" => InstallPlan {
            kind: InstallKind::AppImage,
            program: None,
            args: Vec::new(),
            restart: true,
            description: "Replace the running AppImage and restart".to_owned(),
        },
        "deb" => InstallPlan {
            kind: InstallKind::Debian,
            program: Some(if which("pkexec").is_some() { "pkexec" } else { "dpkg" }.to_owned()),
            args: if which("pkexec").is_some() {
                vec!["dpkg".to_owned(), "-i".to_owned(), display]
            } else {
                vec!["-i".to_owned(), display]
            },
            restart: true,
            description: "Install the Debian package".to_owned(),
        },
        "rpm" => InstallPlan {
            kind: InstallKind::Rpm,
            program: Some(if which("pkexec").is_some() { "pkexec" } else { "rpm" }.to_owned()),
            args: if which("pkexec").is_some() {
                vec!["rpm".to_owned(), "-U".to_owned(), "--replacepkgs".to_owned(), display]
            } else {
                vec!["-U".to_owned(), "--replacepkgs".to_owned(), display]
            },
            restart: true,
            description: "Install the RPM package".to_owned(),
        },
        "msi" if cfg!(windows) => InstallPlan {
            kind: InstallKind::Msi,
            program: Some("msiexec".to_owned()),
            args: vec!["/i".to_owned(), display, "/passive".to_owned()],
            restart: true,
            description: "Install the Windows package".to_owned(),
        },
        "dmg" if cfg!(target_os = "macos") => InstallPlan {
            kind: InstallKind::Dmg,
            program: Some("open".to_owned()),
            args: vec![display],
            restart: false,
            description: "Open the disk image and complete the installer".to_owned(),
        },
        _ => InstallPlan {
            kind: InstallKind::Unsupported,
            program: None,
            args: Vec::new(),
            restart: false,
            description: "No automatic installer is available for this asset".to_owned(),
        },
    }
}

/// Resolve the plan for the current environment.
pub fn current_plan(path: &Path) -> InstallPlan {
    plan_for(path, flatpak_active())
}

/// Install a downloaded asset.
pub fn install(path: &Path) -> Result<InstallOutcome, String> {
    let plan = plan_for(path, flatpak_active());
    match plan.kind {
        InstallKind::AppImage => {
            let target = std::env::var_os("APPIMAGE")
                .map(PathBuf::from)
                .ok_or_else(|| "This build is not running as an AppImage.".to_owned())?;
            install_appimage(path, &target)?;
            Ok(InstallOutcome {
                restart: true,
                message: "The AppImage was replaced. Restart to use the new version.".to_owned(),
            })
        }
        InstallKind::Unsupported => Err(plan.description),
        _ => {
            let program = plan
                .program
                .as_deref()
                .ok_or_else(|| "The installer command is unavailable.".to_owned())?;
            let status = Command::new(program)
                .args(&plan.args)
                .status()
                .map_err(|error| format!("Could not start {program}: {error}"))?;
            if !status.success() {
                return Err(format!("{program} exited with {status}"));
            }
            Ok(InstallOutcome {
                restart: plan.restart,
                message: if plan.restart {
                    "The update was installed. Restart to use the new version.".to_owned()
                } else {
                    "The installer was opened. Complete it, then reopen MD View.".to_owned()
                },
            })
        }
    }
}

/// Replace an AppImage atomically and keep the executable bit.
pub fn install_appimage(download: &Path, target: &Path) -> Result<(), String> {
    if download == target {
        return Err("The downloaded file is the running AppImage.".to_owned());
    }
    let parent = target
        .parent()
        .ok_or_else(|| "The AppImage has no parent directory.".to_owned())?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("mdview.AppImage");
    let temporary = parent.join(format!(".{file_name}.update-{}", std::process::id()));
    fs::copy(download, &temporary).map_err(|error| format!("Could not stage update: {error}"))?;
    set_executable(&temporary)?;
    fs::rename(&temporary, target).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Could not replace the AppImage: {error}")
    })?;
    Ok(())
}

/// Restart the application after installation.
pub fn spawn_restart() -> Result<(), String> {
    let executable = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .ok_or_else(|| "The running executable path is unavailable.".to_owned())?;
    let args = std::env::args_os().skip(1).collect::<Vec<OsString>>();
    restart_command(&executable, &args)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not restart MD View: {error}"))
}

/// Build a detached restart command that waits for this process to exit.
pub fn restart_command(executable: &Path, args: &[OsString]) -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("sleep 1; exec \"$@\"")
            .arg("mdview-restart")
            .arg(executable)
            .args(args);
        command
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("powershell");
        let mut script = String::from("Start-Sleep -Seconds 1; Start-Process -FilePath '");
        script.push_str(&executable.display().to_string().replace('\'', "''"));
        script.push('\'');
        for arg in args {
            script.push_str(" -ArgumentList '");
            script.push_str(&arg.to_string_lossy().replace('\'', "''"));
            script.push('\'');
        }
        command.args(["-NoProfile", "-Command", &script]);
        command
    }
}

fn flatpak_active() -> bool {
    std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").is_file()
}

fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).map_err(|error| error.to_string())?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|error| error.to_string())?;
    }
    let _ = path;
    Ok(())
}

fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(name))
            .find(|path| path.is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_cover_linux_package_formats() {
        let appimage = plan_for(Path::new("MD-View.AppImage"), false);
        assert_eq!(appimage.kind, InstallKind::AppImage);
        assert!(appimage.restart);
        let deb = plan_for(Path::new("mdview_0.9.0_amd64.deb"), false);
        assert_eq!(deb.kind, InstallKind::Debian);
        assert!(deb.args.contains(&"-i".to_owned()) || deb.args.contains(&"dpkg".to_owned()));
        let rpm = plan_for(Path::new("mdview-0.9.0-1.x86_64.rpm"), false);
        assert_eq!(rpm.kind, InstallKind::Rpm);
        assert!(rpm.args.contains(&"--replacepkgs".to_owned()));
        let flatpak = plan_for(Path::new("unused"), true);
        assert_eq!(flatpak.kind, InstallKind::Flatpak);
        assert_eq!(flatpak.args, ["update", "-y"]);
    }

    #[test]
    fn unsupported_assets_do_not_install() {
        let plan = plan_for(Path::new("mdview.tar.gz"), false);
        assert_eq!(plan.kind, InstallKind::Unsupported);
        assert!(!plan.restart);
    }

    #[test]
    fn appimage_replacement_is_atomic_and_executable() {
        let directory = tempfile::tempdir().expect("tempdir");
        let target = directory.path().join("mdview.AppImage");
        let download = directory.path().join("download.AppImage");
        fs::write(&target, b"old").expect("old");
        fs::write(&download, b"new").expect("new");
        install_appimage(&download, &target).expect("install");
        assert_eq!(fs::read(&target).expect("read"), b"new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&target).expect("metadata").permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
        assert!(
            !directory
                .path()
                .read_dir()
                .expect("dir")
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().contains(".update-"))
        );
    }

    #[test]
    fn restart_command_uses_a_detached_waiter() {
        let command = restart_command(Path::new("/tmp/mdview"), &[OsString::from("--flag")]);
        assert_eq!(command.get_program().to_string_lossy(), "sh");
        #[cfg(unix)]
        {
            let args = command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            assert_eq!(args[0], "-c");
            assert!(args[1].contains("sleep 1"));
            assert_eq!(args[3], "/tmp/mdview");
            assert_eq!(args[4], "--flag");
        }
    }
}
