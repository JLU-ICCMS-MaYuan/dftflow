#!/usr/bin/env python3
import os
import shlex


def resolve_vasp_command(command):
    value = str(command).strip() if command is not None else ""
    return value or "vasp_std"


def resolve_direct_command(command, default_command):
    value = str(command).strip() if command is not None else ""
    return value or default_command


def resolve_qe_command(command_or_bin, exe_name):
    value = str(command_or_bin).strip() if command_or_bin is not None else ""
    if not value:
        return exe_name

    tokens = shlex.split(value)
    if not tokens:
        return exe_name

    expanded_tokens = [os.path.expandvars(os.path.expanduser(token)) for token in tokens]
    target = expanded_tokens[-1]
    if os.path.basename(target) == exe_name:
        executable = target
    else:
        executable = os.path.join(target, exe_name)

    rebuilt_tokens = [shlex.quote(token) for token in expanded_tokens[:-1]]
    rebuilt_tokens.append(shlex.quote(executable))
    return " ".join(rebuilt_tokens)
