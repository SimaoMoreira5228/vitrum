from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent

CARGO_RELEASE_PATH = PROJECT_ROOT / "target" / "release"
VITRUM_BINARY = CARGO_RELEASE_PATH / "vitrum"
VITRUM_BAR_BINARY = CARGO_RELEASE_PATH / "vitrum-bar"
VITRUM_CTL_BINARY = CARGO_RELEASE_PATH / "vitrumctl"
VITRUM_CLIP_BINARY = CARGO_RELEASE_PATH / "vitrum-clip"
VITRUM_NOTIFY_BINARY = CARGO_RELEASE_PATH / "vitrum-notify"
VITRUM_KEYRING_BINARY = CARGO_RELEASE_PATH / "vitrum-keyring"

INSTALL_PREFIX = Path("/usr")
BIN_DIR = INSTALL_PREFIX / "bin"
WAYLAND_SESSIONS_DIR = INSTALL_PREFIX / "share/wayland-sessions"
LIGHTDM_DIR = INSTALL_PREFIX / "share/lightdm"
DOC_DIR = INSTALL_PREFIX / "share/doc/vitrum"
LIGHTDM_CONFIG_DIR = Path("/etc/lightdm")

ASSETS_DIR = PROJECT_ROOT / "installer/assets"
SESSION_DESKTOP = ASSETS_DIR / "vitrum.desktop"
SESSION_WAYLAND_DESKTOP = ASSETS_DIR / "lightdm/vitrum-wayland.desktop"
SESSION_SCRIPT = ASSETS_DIR / "lightdm/vitrum-session"
SETUP_SCRIPT = ASSETS_DIR / "lightdm/vitrum-setup"
LIGHTDM_CONFIG = ASSETS_DIR / "lightdm/lightdm-vitrum.conf"
README_SOURCE = PROJECT_ROOT / "README.md"
LICENSE_SOURCE = PROJECT_ROOT / "LICENSE"

DEFAULT_FEATURES = []
AVAILABLE_FEATURES = {
    "bar": "vitrum-bar",
    "ctl": "vitrumctl",
    "clip": "vitrum-clip",
    "notify": "vitrum-notify",
    "keyring": "vitrum-keyring",
}

RUNTIME_DEPS = {
    "arch": {
        "base": [
            "seatd",
            "libinput",
            "mesa",
            "vulkan-icd-loader",
            "wayland",
            "libxkbcommon",
            "dbus",
            "xorg-xwayland",
        ],
        "bar": [],
    },
    "fedora": {
        "base": [
            "libseat",
            "libinput",
            "mesa-libgbm",
            "mesa-libEGL",
            "vulkan-loader",
            "wayland",
            "libxkbcommon",
            "dbus",
            "xorg-xwayland",
        ],
        "bar": [],
    },
    "debian": {
        "base": [
            "libseat1",
            "libinput10",
            "libgbm1",
            "libegl1",
            "libvulkan1",
            "libwayland0",
            "libxkbcommon0",
            "libdbus-1-3",
            "xwayland",
        ],
        "bar": [],
    },
}

BUILD_DEPS = {
    "arch": {
        "base": [
            "rustup",
            "cargo",
            "cmake",
            "pkg-config",
            "libgit2",
        ],
        "bar": [],
    },
    "fedora": {
        "base": [
            "rust",
            "cargo",
            "cmake",
            "pkg-config",
            "libseat-devel",
            "libinput-devel",
            "mesa-libgbm-devel",
            "mesa-libEGL-devel",
            "mesa-libGL-devel",
            "vulkan-loader-devel",
            "wayland-devel",
            "wayland-protocols-devel",
            "libxkbcommon-devel",
            "dbus-devel",
            "libgit2-devel",
        ],
        "bar": [],
    },
    "debian": {
        "base": [
            "cargo",
            "cmake",
            "pkg-config",
            "libseat-dev",
            "libinput-dev",
            "libgbm-dev",
            "libegl1-mesa-dev",
            "libgl1-mesa-dev",
            "libvulkan-dev",
            "libwayland-dev",
            "wayland-protocols",
            "libxkbcommon-dev",
            "libdbus-1-dev",
            "libgit2-dev",
        ],
        "bar": [],
    },
}

REQUIRED_GROUPS = {
    "seat": "Required by libseat for TTY access",
}
