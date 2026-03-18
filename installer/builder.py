from .config import PROJECT_ROOT
from .utils import (
    log_info,
    log_success,
    log_error,
    log_section,
    run_command,
)


class Builder:
    def __init__(self):
        self.project_root = PROJECT_ROOT

    def build(
        self,
        with_bar: bool = False,
        with_ctl: bool = False,
        with_clip: bool = False,
        with_notify: bool = False,
        with_keyring: bool = False,
        release: bool = True,
    ) -> bool:
        log_section("Building Vitrum")

        args = ["cargo", "build"]

        if release:
            args.append("--release")

        packages = ["vitrum"]
        if with_bar:
            packages.append("vitrum-bar")
        if with_ctl:
            packages.append("vitrumctl")
        if with_clip:
            packages.append("vitrum-clip")
        if with_notify:
            packages.append("vitrum-notify")
        if with_keyring:
            packages.append("vitrum-keyring")

        for package in packages:
            args.extend(["--package", package])

        log_info("Building vitrum compositor...")
        try:
            run_command(
                args,
                check=True,
                description="Compiling vitrum (this may take a while)...",
            )
            log_success("Vitrum build completed")
        except Exception as e:
            log_error(f"Build failed: {e}")
            return False

        if with_bar:
            log_success("vitrum-bar build completed")
        if with_ctl:
            log_success("vitrumctl build completed")
        if with_clip:
            log_success("vitrum-clip build completed")
        if with_notify:
            log_success("vitrum-notify build completed")
        if with_keyring:
            log_success("vitrum-keyring build completed")

        return True

    def clean(self) -> bool:
        log_info("Cleaning build artifacts...")
        try:
            run_command(
                ["cargo", "clean"],
                check=True,
                cwd=self.project_root,
            )
            log_success("Build artifacts cleaned")
            return True
        except Exception as e:
            log_error(f"Cleanup failed: {e}")
            return False

    def verify_build(
        self,
        with_bar: bool = False,
        with_ctl: bool = False,
        with_clip: bool = False,
        with_notify: bool = False,
        with_keyring: bool = False,
    ) -> bool:
        from .config import (
            VITRUM_BINARY,
            VITRUM_BAR_BINARY,
            VITRUM_CTL_BINARY,
            VITRUM_CLIP_BINARY,
            VITRUM_NOTIFY_BINARY,
            VITRUM_KEYRING_BINARY,
        )

        log_info("Verifying build artifacts...")

        if not VITRUM_BINARY.exists():
            log_error(f"Vitrum binary not found at {VITRUM_BINARY}")
            return False
        log_success(f"Found vitrum binary: {VITRUM_BINARY}")

        if with_bar:
            if not VITRUM_BAR_BINARY.exists():
                log_error(f"Vitrum-bar binary not found at {VITRUM_BAR_BINARY}")
                return False
            log_success(f"Found vitrum-bar binary: {VITRUM_BAR_BINARY}")

        if with_ctl:
            if not VITRUM_CTL_BINARY.exists():
                log_error(f"Vitrumctl binary not found at {VITRUM_CTL_BINARY}")
                return False
            log_success(f"Found vitrumctl binary: {VITRUM_CTL_BINARY}")

        if with_clip:
            if not VITRUM_CLIP_BINARY.exists():
                log_error(f"Vitrum-clip binary not found at {VITRUM_CLIP_BINARY}")
                return False
            log_success(f"Found vitrum-clip binary: {VITRUM_CLIP_BINARY}")

        if with_notify:
            if not VITRUM_NOTIFY_BINARY.exists():
                log_error(f"Vitrum-notify binary not found at {VITRUM_NOTIFY_BINARY}")
                return False
            log_success(f"Found vitrum-notify binary: {VITRUM_NOTIFY_BINARY}")

        if with_keyring:
            if not VITRUM_KEYRING_BINARY.exists():
                log_error(f"Vitrum-keyring binary not found at {VITRUM_KEYRING_BINARY}")
                return False
            log_success(f"Found vitrum-keyring binary: {VITRUM_KEYRING_BINARY}")

        return True
