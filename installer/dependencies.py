from typing import List, Set, Optional

from .config import (
    RUNTIME_DEPS,
    BUILD_DEPS,
    VITRUM_BINARY,
    VITRUM_BAR_BINARY,
)
from .utils import (
    log_info,
    log_success,
    log_error,
    log_warning,
    run_command,
    run_sudo_command,
    command_exists,
    prompt_confirm,
    detect_distribution,
    get_package_manager,
)


class DependencyManager:
    def __init__(self, distro: Optional[str] = None, install_missing: bool = False):
        self.distro = distro or detect_distribution()
        self.install_missing = install_missing
        self.missing_runtime_deps: Set[str] = set()
        self.missing_build_deps: Set[str] = set()

    def check_build_dependencies(
        self,
        with_bar: bool = False,
        with_ctl: bool = False,
        with_clip: bool = False,
        with_notify: bool = False,
    ) -> bool:
        if not self.distro:
            log_error(
                "Could not detect distribution. Please install build dependencies manually."
            )
            return False

        log_info(f"Detected distribution: {self.distro}")

        required_tools = ["cargo", "cmake", "pkg-config"]
        self.missing_build_deps = set()

        for tool in required_tools:
            if not command_exists(tool):
                self.missing_build_deps.add(tool)

        deps_to_check = BUILD_DEPS.get(self.distro, {}).get("base", [])
        if with_bar:
            deps_to_check = list(
                set(deps_to_check) | set(BUILD_DEPS.get(self.distro, {}).get("bar", []))
            )
        if with_ctl:
            deps_to_check = list(
                set(deps_to_check) | set(BUILD_DEPS.get(self.distro, {}).get("ctl", []))
            )
        if with_clip:
            deps_to_check = list(
                set(deps_to_check)
                | set(BUILD_DEPS.get(self.distro, {}).get("clip", []))
            )
        if with_notify:
            deps_to_check = list(
                set(deps_to_check)
                | set(BUILD_DEPS.get(self.distro, {}).get("notify", []))
            )

        for dep in deps_to_check:
            if not self._check_package_installed(dep):
                self.missing_build_deps.add(dep)

        if not self.missing_build_deps:
            log_success("All build dependencies are installed")
            return True

        log_warning(
            f"Missing build dependencies: {', '.join(sorted(self.missing_build_deps))}"
        )

        if self.install_missing:
            return self._install_packages(list(self.missing_build_deps))

        return False

    def check_runtime_dependencies(
        self,
        with_bar: bool = False,
        with_ctl: bool = False,
        with_clip: bool = False,
        with_notify: bool = False,
    ) -> bool:
        if not self.distro:
            log_error(
                "Could not detect distribution. Please install runtime dependencies manually."
            )
            return False

        deps_to_check = RUNTIME_DEPS.get(self.distro, {}).get("base", [])
        if with_bar:
            deps_to_check = list(
                set(deps_to_check)
                | set(RUNTIME_DEPS.get(self.distro, {}).get("bar", []))
            )
        if with_ctl:
            deps_to_check = list(
                set(deps_to_check)
                | set(RUNTIME_DEPS.get(self.distro, {}).get("ctl", []))
            )
        if with_clip:
            deps_to_check = list(
                set(deps_to_check)
                | set(RUNTIME_DEPS.get(self.distro, {}).get("clip", []))
            )
        if with_notify:
            deps_to_check = list(
                set(deps_to_check)
                | set(RUNTIME_DEPS.get(self.distro, {}).get("notify", []))
            )

        self.missing_runtime_deps = set()
        for dep in deps_to_check:
            if not self._check_package_installed(dep):
                self.missing_runtime_deps.add(dep)

        if not self.missing_runtime_deps:
            log_success("All runtime dependencies are installed")
            return True

        log_warning(
            f"Missing runtime dependencies: {', '.join(sorted(self.missing_runtime_deps))}"
        )

        if self.install_missing:
            return self._install_packages(list(self.missing_runtime_deps))

        return False

    def _check_package_installed(self, package: str) -> bool:
        if self.distro == "arch":
            _, stdout, _ = run_command(
                ["pacman", "-Q", package],
                check=False,
                capture_output=True,
            )
            return "not found" not in stdout.lower()
        elif self.distro == "fedora":
            _, stdout, _ = run_command(
                ["rpm", "-q", package],
                check=False,
                capture_output=True,
            )
            return "is not installed" not in stdout.lower()
        elif self.distro == "debian":
            return_code, _, _ = run_command(
                ["dpkg", "-l"],
                check=False,
                capture_output=True,
            )
            return return_code == 0
        return False

    def _install_packages(self, packages: List[str]) -> bool:
        if not self.distro:
            log_error("Cannot install packages: distribution not detected")
            return False

        pm = get_package_manager(self.distro)
        if not pm:
            log_error(f"No package manager found for {self.distro}")
            return False

        if not prompt_confirm(f"Install {len(packages)} missing packages?"):
            return False

        if self.distro == "arch":
            cmd = ["pacman", "-S", "--noconfirm"] + packages
        elif self.distro == "fedora":
            cmd = ["dnf", "install", "-y"] + packages
        elif self.distro == "debian":
            cmd = ["apt", "install", "-y"] + packages
        else:
            return False

        try:
            run_sudo_command(cmd)
            log_success("Dependencies installed successfully")
            return True
        except Exception as e:
            log_error(f"Failed to install dependencies: {e}")
            return False

    def check_binaries_exist(self, with_bar: bool = False) -> bool:
        if not VITRUM_BINARY.exists():
            return False
        if with_bar and not VITRUM_BAR_BINARY.exists():
            return False
        return True

    def print_summary(self) -> None:
        if self.missing_build_deps or self.missing_runtime_deps:
            log_warning("\nDependency Summary:")
            if self.missing_build_deps:
                log_warning(f"  Build: {', '.join(sorted(self.missing_build_deps))}")
            if self.missing_runtime_deps:
                log_warning(
                    f"  Runtime: {', '.join(sorted(self.missing_runtime_deps))}"
                )
        else:
            log_success("\nAll dependencies are satisfied!")
