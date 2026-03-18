import argparse
import sys

from .utils import log_section, log_error, log_success, log_warning
from .dependencies import DependencyManager
from .groups import GroupManager
from .builder import Builder
from .installer import FileInstaller


class VitrumInstaller:
    def __init__(self):
        self.deps = DependencyManager()
        self.groups = GroupManager()
        self.builder = Builder()
        self.file_installer = FileInstaller()

    def run_install(self, args) -> bool:
        if not self.deps.check_build_dependencies(
            with_bar=args.bar,
            with_ctl=args.ctl,
            with_clip=args.clip,
            with_notify=args.notify,
            with_keyring=args.keyring,
        ):
            self.deps.print_summary()
            if not args.skip_deps:
                return False
            log_warning("Skipping dependency check as requested")

        from .config import SESSION_DESKTOP, LICENSE_SOURCE

        missing_files = [
            SESSION_DESKTOP,
            LICENSE_SOURCE,
        ]

        for file_path in missing_files:
            if not file_path.exists():
                log_error(f"Required file missing: {file_path}")
                return False

        if not args.skip_build:
            if not self.builder.build(
                with_bar=args.bar,
                with_ctl=args.ctl,
                with_clip=args.clip,
                with_notify=args.notify,
                with_keyring=args.keyring,
                release=args.release,
            ):
                return False

        if not self.builder.verify_build(
            with_bar=args.bar,
            with_ctl=args.ctl,
            with_clip=args.clip,
            with_notify=args.notify,
            with_keyring=args.keyring,
        ):
            return False

        if not self.deps.check_runtime_dependencies(
            with_bar=args.bar,
            with_ctl=args.ctl,
            with_clip=args.clip,
            with_notify=args.notify,
            with_keyring=args.keyring,
        ):
            self.deps.print_summary()
            if not args.skip_deps:
                log_warning("Install runtime dependencies and try again")
                return False

        if not self.groups.check_groups():
            if not args.skip_groups:
                self.groups.print_summary()
                log_warning("Setup groups and try again")
                return False
            log_warning("Skipping group setup as requested")

        if not args.skip_install:
            if not self.file_installer.install(
                with_bar=args.bar,
                with_ctl=args.ctl,
                with_clip=args.clip,
                with_notify=args.notify,
                with_keyring=args.keyring,
            ):
                return False
            self.file_installer.print_summary()

        log_success("\n✓ Vitrum installation completed successfully!")

        if self.groups.user_not_in_groups:
            log_warning(
                "\nYou need to log out and back in for group changes to take effect,\nor run: exec su -l $USER"
            )

        return True

    def run_uninstall(self, args) -> bool:
        log_section("Vitrum Uninstaller")

        if not self.file_installer.uninstall():
            return False

        log_success("Vitrum uninstalled successfully")
        return True

    def run_check(self, args) -> bool:
        log_section("Vitrum Dependency Check")

        self.deps.check_build_dependencies(
            with_bar=args.bar,
            with_ctl=args.ctl,
            with_clip=args.clip,
            with_notify=args.notify,
            with_keyring=args.keyring,
        )
        self.deps.print_summary()

        self.deps.check_runtime_dependencies(
            with_bar=args.bar,
            with_ctl=args.ctl,
            with_clip=args.clip,
            with_notify=args.notify,
            with_keyring=args.keyring,
        )
        self.deps.print_summary()

        self.groups.check_groups()
        self.groups.print_summary()

        return True


def main():
    parser = argparse.ArgumentParser(
        prog="vitrum-installer",
        description="Vitrum Wayland compositor installer",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  vitrum-installer install                 # Full installation
  vitrum-installer install --bar           # Install with vitrum-bar
  vitrum-installer install --ctl           # Install with vitrumctl
  vitrum-installer install --bar --ctl     # Install compositor + bar + ctl
  vitrum-installer check                   # Check dependencies only
  vitrum-installer uninstall               # Remove Vitrum
        """,
    )

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    install_parser = subparsers.add_parser(
        "install",
        help="Build and install Vitrum",
    )
    install_parser.add_argument(
        "--bar",
        action="store_true",
        help="Include vitrum-bar in build",
    )
    install_parser.add_argument(
        "--ctl",
        action="store_true",
        help="Include vitrumctl in build",
    )
    install_parser.add_argument(
        "--clip",
        action="store_true",
        help="Include vitrum-clip in build",
    )
    install_parser.add_argument(
        "--notify",
        action="store_true",
        help="Include vitrum-notify in build",
    )
    install_parser.add_argument(
        "--keyring",
        action="store_true",
        help="Include vitrum-keyring in build",
    )
    install_parser.add_argument(
        "--skip-build",
        action="store_true",
        help="Skip compilation (use existing binaries)",
    )
    install_parser.add_argument(
        "--skip-install",
        action="store_true",
        help="Skip file installation",
    )
    install_parser.add_argument(
        "--skip-deps",
        action="store_true",
        help="Skip dependency checking",
    )
    install_parser.add_argument(
        "--skip-groups",
        action="store_true",
        help="Skip group setup",
    )
    install_parser.add_argument(
        "--debug",
        action="store_true",
        help="Debug build (no optimization)",
    )
    install_parser.set_defaults(func=lambda args: VitrumInstaller().run_install(args))

    uninstall_parser = subparsers.add_parser(
        "uninstall",
        help="Remove Vitrum",
    )
    uninstall_parser.set_defaults(
        func=lambda args: VitrumInstaller().run_uninstall(args)
    )

    check_parser = subparsers.add_parser(
        "check",
        help="Check dependencies",
    )
    check_parser.add_argument(
        "--bar",
        action="store_true",
        help="Check dependencies for vitrum-bar",
    )
    check_parser.add_argument(
        "--ctl",
        action="store_true",
        help="Check dependencies for vitrumctl",
    )
    check_parser.add_argument(
        "--clip",
        action="store_true",
        help="Check dependencies for vitrum-clip",
    )
    check_parser.add_argument(
        "--notify",
        action="store_true",
        help="Check dependencies for vitrum-notify",
    )
    check_parser.add_argument(
        "--keyring",
        action="store_true",
        help="Check dependencies for vitrum-keyring",
    )
    check_parser.set_defaults(func=lambda args: VitrumInstaller().run_check(args))

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 1

    if args.command == "install" and hasattr(args, "debug"):
        args.release = not args.debug
    else:
        args.release = True

    try:
        success = args.func(args)
        return 0 if success else 1
    except KeyboardInterrupt:
        print("\n\nInstallation cancelled by user")
        return 1
    except Exception as e:
        log_error(f"Unexpected error: {e}")
        import traceback

        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
