from pathlib import Path
from typing import List

from .config import (
    VITRUM_BINARY,
    VITRUM_BAR_BINARY,
    VITRUM_CTL_BINARY,
    VITRUM_CLIP_BINARY,
    VITRUM_NOTIFY_BINARY,
    VITRUM_KEYRING_BINARY,
    BIN_DIR,
    WAYLAND_SESSIONS_DIR,
    LIGHTDM_DIR,
    DOC_DIR,
    LIGHTDM_CONFIG_DIR,
    SESSION_DESKTOP,
    SESSION_WAYLAND_DESKTOP,
    SESSION_SCRIPT,
    SETUP_SCRIPT,
    LIGHTDM_CONFIG,
    README_SOURCE,
    LICENSE_SOURCE,
)
from .utils import (
    log_info,
    log_success,
    log_error,
    log_section,
    run_sudo_command,
)


class FileInstaller:
    def __init__(self):
        self.installed_files: List[Path] = []

    def install(
        self,
        with_bar: bool = False,
        with_ctl: bool = False,
        with_clip: bool = False,
        with_notify: bool = False,
        with_keyring: bool = False,
    ) -> bool:
        log_section("Installing Vitrum")

        installation_steps = [
            (
                lambda: self._install_binaries(
                    with_bar=with_bar,
                    with_ctl=with_ctl,
                    with_clip=with_clip,
                    with_notify=with_notify,
                    with_keyring=with_keyring,
                ),
                "Installing binaries",
            ),
            (self._install_session_files, "Installing Wayland session files"),
            (self._install_lightdm_integration, "Installing LightDM integration"),
            (self._install_documentation, "Installing documentation"),
        ]

        for step_func, description in installation_steps:
            log_info(description)
            try:
                if not step_func():
                    return False
            except Exception as e:
                log_error(f"Installation failed: {e}")
                return False

        log_success("Vitrum installation completed")
        return True

    def _install_binaries(
        self,
        with_bar: bool = False,
        with_ctl: bool = False,
        with_clip: bool = False,
        with_notify: bool = False,
        with_keyring: bool = False,
    ) -> bool:
        if not VITRUM_BINARY.exists():
            log_error(f"Vitrum binary not found at {VITRUM_BINARY}")
            return False

        self._create_directory(BIN_DIR)
        if not self._copy_file(VITRUM_BINARY, BIN_DIR / "vitrum", mode=0o755):
            return False

        if with_bar:
            if not VITRUM_BAR_BINARY.exists():
                log_error(f"Vitrum-bar binary not found at {VITRUM_BAR_BINARY}")
                return False
            if not self._copy_file(
                VITRUM_BAR_BINARY, BIN_DIR / "vitrum-bar", mode=0o755
            ):
                return False

        if with_ctl:
            if not VITRUM_CTL_BINARY.exists():
                log_error(f"Vitrumctl binary not found at {VITRUM_CTL_BINARY}")
                return False
            if not self._copy_file(
                VITRUM_CTL_BINARY, BIN_DIR / "vitrumctl", mode=0o755
            ):
                return False

        if with_clip:
            if not VITRUM_CLIP_BINARY.exists():
                log_error(f"Vitrum-clip binary not found at {VITRUM_CLIP_BINARY}")
                return False
            if not self._copy_file(
                VITRUM_CLIP_BINARY, BIN_DIR / "vitrum-clip", mode=0o755
            ):
                return False

        if with_notify:
            if not VITRUM_NOTIFY_BINARY.exists():
                log_error(f"Vitrum-notify binary not found at {VITRUM_NOTIFY_BINARY}")
                return False
            if not self._copy_file(
                VITRUM_NOTIFY_BINARY, BIN_DIR / "vitrum-notify", mode=0o755
            ):
                return False

        if with_keyring:
            if not VITRUM_KEYRING_BINARY.exists():
                log_error(f"Vitrum-keyring binary not found at {VITRUM_KEYRING_BINARY}")
                return False
            if not self._copy_file(
                VITRUM_KEYRING_BINARY, BIN_DIR / "vitrum-keyring", mode=0o755
            ):
                return False

        return True

    def _install_session_files(self) -> bool:
        files_to_install = [
            (SESSION_DESKTOP, WAYLAND_SESSIONS_DIR / "vitrum.desktop", 0o644),
            (
                SESSION_WAYLAND_DESKTOP,
                WAYLAND_SESSIONS_DIR / "vitrum-wayland.desktop",
                0o644,
            ),
            (SESSION_SCRIPT, LIGHTDM_DIR / "vitrum-session", 0o755),
            (SETUP_SCRIPT, LIGHTDM_DIR / "vitrum-setup", 0o755),
        ]

        for src, dst, mode in files_to_install:
            if not src.exists():
                log_error(f"File not found: {src}")
                return False

            self._create_directory(dst.parent)
            if not self._copy_file(src, dst, mode=mode):
                return False

        return True

    def _install_lightdm_integration(self) -> bool:
        if not LIGHTDM_CONFIG.exists():
            log_error(f"LightDM config not found at {LIGHTDM_CONFIG}")
            return False

        self._create_directory(LIGHTDM_CONFIG_DIR)
        return self._copy_file(
            LIGHTDM_CONFIG, LIGHTDM_CONFIG_DIR / "lightdm-vitrum.conf", mode=0o644
        )

    def _install_documentation(self) -> bool:
        files_to_install = [
            (README_SOURCE, DOC_DIR / "README.md"),
            (LICENSE_SOURCE, DOC_DIR / "LICENSE"),
        ]

        for src, dst in files_to_install:
            if not src.exists():
                log_error(f"File not found: {src}")
                return False

            self._create_directory(dst.parent)
            if not self._copy_file(src, dst, mode=0o644):
                return False

        return True

    def _create_directory(self, path: Path) -> bool:
        if path.exists() and path.is_dir():
            return True

        try:
            run_sudo_command(
                ["mkdir", "-p", str(path)],
                check=True,
                description=f"Creating directory {path}...",
            )
            return True
        except Exception as e:
            log_error(f"Failed to create directory {path}: {e}")
            return False

    def _copy_file(self, src: Path, dst: Path, mode: int = 0o644) -> bool:
        if not src.exists():
            log_error(f"Source file not found: {src}")
            return False

        try:
            run_sudo_command(
                ["cp", str(src), str(dst)],
                check=True,
                description=f"Copying {src.name}...",
            )

            run_sudo_command(
                ["chmod", format(mode, "o"), str(dst)],
                check=True,
            )

            self.installed_files.append(dst)
            log_success(f"Installed {src.name}")
            return True
        except Exception as e:
            log_error(f"Failed to install {src.name}: {e}")
            return False

    def uninstall(self) -> bool:
        log_section("Uninstalling Vitrum")

        files_to_remove = [
            BIN_DIR / "vitrum",
            BIN_DIR / "vitrum-bar",
            BIN_DIR / "vitrumctl",
            BIN_DIR / "vitrum-clip",
            BIN_DIR / "vitrum-notify",
            BIN_DIR / "vitrum-keyring",
            WAYLAND_SESSIONS_DIR / "vitrum.desktop",
            WAYLAND_SESSIONS_DIR / "vitrum-wayland.desktop",
            LIGHTDM_DIR / "vitrum-session",
            LIGHTDM_DIR / "vitrum-setup",
            LIGHTDM_CONFIG_DIR / "lightdm-vitrum.conf",
            DOC_DIR,
        ]

        for file_path in files_to_remove:
            if file_path.exists():
                try:
                    run_sudo_command(
                        ["rm", "-rf", str(file_path)],
                        check=True,
                        description=f"Removing {file_path}...",
                    )
                    log_success(f"Removed {file_path}")
                except Exception as e:
                    log_error(f"Failed to remove {file_path}: {e}")
                    return False

        log_success("Vitrum uninstalled")
        return True

    def print_summary(self) -> None:
        if self.installed_files:
            print("\nInstalled files:")
            for file_path in self.installed_files:
                print(f"  {file_path}")
