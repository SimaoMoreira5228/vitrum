import sys
import subprocess
from pathlib import Path
from typing import Optional, List, Tuple
import shutil


class Colors:
    RESET = "\033[0m"
    BOLD = "\033[1m"
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    BLUE = "\033[94m"
    CYAN = "\033[96m"


def log_info(message: str) -> None:
    print(f"{Colors.BLUE}ℹ{Colors.RESET} {message}")


def log_success(message: str) -> None:
    print(f"{Colors.GREEN}✓{Colors.RESET} {message}")


def log_warning(message: str) -> None:
    print(f"{Colors.YELLOW}⚠{Colors.RESET} {message}")


def log_error(message: str) -> None:
    print(f"{Colors.RED}✗{Colors.RESET} {message}", file=sys.stderr)


def log_section(title: str) -> None:
    print(f"\n{Colors.BOLD}{Colors.CYAN}=== {title} ==={Colors.RESET}\n")


def run_command(
    cmd: List[str],
    check: bool = True,
    capture_output: bool = False,
    description: Optional[str] = None,
) -> Tuple[int, str, str]:
    if description:
        log_info(description)

    try:
        result = subprocess.run(
            cmd,
            check=False,
            capture_output=capture_output,
            text=True,
        )

        if check and result.returncode != 0:
            if result.stderr:
                log_error(result.stderr)
            raise RuntimeError(f"Command failed: {' '.join(cmd)}")

        return result.returncode, result.stdout, result.stderr
    except FileNotFoundError:
        log_error(f"Command not found: {cmd[0]}")
        raise


def run_sudo_command(
    cmd: List[str],
    check: bool = True,
    description: Optional[str] = None,
) -> Tuple[int, str, str]:
    return run_command(["sudo"] + cmd, check=check, description=description)


def command_exists(cmd: str) -> bool:
    return shutil.which(cmd) is not None


def file_exists(path: Path) -> bool:
    return path.exists()


def prompt_confirm(message: str, default: bool = True) -> bool:
    default_text = "Y/n" if default else "y/N"
    response = (
        input(f"{Colors.YELLOW}?{Colors.RESET} {message} [{default_text}]: ")
        .strip()
        .lower()
    )

    if response == "":
        return default
    return response in ("y", "yes")


def prompt_choice(message: str, choices: List[str]) -> str:
    print(f"\n{Colors.YELLOW}?{Colors.RESET} {message}")
    for i, choice in enumerate(choices, 1):
        print(f"  {i}) {choice}")

    while True:
        try:
            selected = int(input("Select (number): ").strip())
            if 1 <= selected <= len(choices):
                return choices[selected - 1]
        except (ValueError, IndexError):
            pass
        print("Invalid selection, please try again.")


def detect_distribution() -> Optional[str]:
    if Path("/etc/os-release").exists():
        with open("/etc/os-release") as f:
            content = f.read().lower()
            if "arch" in content or "manjaro" in content:
                return "arch"
            elif "fedora" in content or "rhel" in content or "centos" in content:
                return "fedora"
            elif "debian" in content or "ubuntu" in content:
                return "debian"

    return None


def get_package_manager(distro: str) -> Optional[str]:
    managers = {
        "arch": "pacman",
        "fedora": "dnf",
        "debian": "apt",
    }
    return managers.get(distro)
