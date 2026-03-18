import grp
import pwd
import os
from typing import Set

from .config import REQUIRED_GROUPS
from .utils import (
    log_info,
    log_success,
    log_warning,
    log_error,
    run_sudo_command,
    prompt_confirm,
)


class GroupManager:
    def __init__(self):
        self.current_user = os.getenv("USER", os.getenv("LOGNAME", "unknown"))
        self.missing_groups: Set[str] = set()
        self.user_not_in_groups: Set[str] = set()

    def check_groups(self) -> bool:
        log_info("Checking user groups...")

        user_groups = self._get_user_groups()
        self.missing_groups = set()
        self.user_not_in_groups = set()

        for group_name, description in REQUIRED_GROUPS.items():
            if not self._group_exists(group_name):
                self.missing_groups.add(group_name)
                log_warning(f"Group '{group_name}' does not exist - {description}")
            elif group_name not in user_groups:
                self.user_not_in_groups.add(group_name)
                log_warning(
                    f"User '{self.current_user}' is not in group '{group_name}' - {description}"
                )
            else:
                log_success(f"User is in group '{group_name}'")

        if self.missing_groups or self.user_not_in_groups:
            return False

        log_success("All group requirements satisfied")
        return True

    def setup_groups(self) -> bool:
        if not self.missing_groups and not self.user_not_in_groups:
            return True

        if self.missing_groups:
            log_warning(
                f"Need to create groups: {', '.join(sorted(self.missing_groups))}"
            )
            if not prompt_confirm("Create missing groups?"):
                return False

            for group in self.missing_groups:
                try:
                    run_sudo_command(
                        ["groupadd", "-r", group],
                        description=f"Creating group '{group}'...",
                    )
                    log_success(f"Created group '{group}'")
                except Exception as e:
                    log_error(f"Failed to create group '{group}': {e}")
                    return False

        if self.user_not_in_groups:
            log_warning(
                f"Need to add user '{self.current_user}' to groups: {', '.join(sorted(self.user_not_in_groups))}"
            )
            if not prompt_confirm(f"Add '{self.current_user}' to these groups?"):
                return False

            for group in self.user_not_in_groups:
                try:
                    run_sudo_command(
                        ["usermod", "-a", "-G", group, self.current_user],
                        description=f"Adding '{self.current_user}' to group '{group}'...",
                    )
                    log_success(f"Added '{self.current_user}' to group '{group}'")
                except Exception as e:
                    log_error(f"Failed to add user to group '{group}': {e}")
                    return False

            log_warning(
                f"\nGroup membership will take effect after you log out and back in.\nTo activate immediately, run: newgrp {list(self.user_not_in_groups)[0]}"
            )

        return True

    def _group_exists(self, group_name: str) -> bool:
        try:
            grp.getgrnam(group_name)
            return True
        except KeyError:
            return False

    def _get_user_groups(self) -> Set[str]:
        try:
            user_info = pwd.getpwnam(self.current_user)
            groups = {grp.getgrgid(user_info.pw_gid).gr_name}

            for group_info in grp.getgrall():
                if self.current_user in group_info.gr_mem:
                    groups.add(group_info.gr_name)

            return groups
        except KeyError:
            return set()

    def print_summary(self) -> None:
        if self.missing_groups or self.user_not_in_groups:
            log_warning("\nGroup Summary:")
            if self.missing_groups:
                log_warning(
                    f"  Missing groups: {', '.join(sorted(self.missing_groups))}"
                )
            if self.user_not_in_groups:
                log_warning(
                    f"  User not in: {', '.join(sorted(self.user_not_in_groups))}"
                )
        else:
            log_success("\nAll group requirements satisfied!")
