#!/usr/bin/env python3
import os
from dataclasses import dataclass
from typing import Iterable


ERROR_MARKERS = (
    "%%%%%%%%%%%%%%",
    "error in routine",
    "error:",
    "stopping ...",
    "convergence not achieved",
)


def _read_if_exists(path: str) -> str:
    if not os.path.exists(path):
        return ""
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        return handle.read()


def _has_error_marker(text: str, extra_markers: Iterable[str] = ()) -> bool:
    lowered = text.lower()
    markers = tuple(marker.lower() for marker in ERROR_MARKERS) + tuple(
        marker.lower() for marker in extra_markers
    )
    return any(marker in lowered for marker in markers)


@dataclass
class ScfOut:
    path: str

    def exists(self) -> bool:
        return os.path.exists(self.path)

    def text(self) -> str:
        return _read_if_exists(self.path)

    def is_finished(self) -> bool:
        return "JOB DONE" in self.text()

    def has_error(self) -> bool:
        text = self.text()
        if not text:
            return False
        return _has_error_marker(text)


@dataclass
class RelaxOut:
    path: str

    def exists(self) -> bool:
        return os.path.exists(self.path)

    def text(self) -> str:
        return _read_if_exists(self.path)

    def is_finished(self) -> bool:
        text = self.text()
        return "JOB DONE" in text and "End final coordinates" in text

    def has_error(self) -> bool:
        text = self.text()
        if not text:
            return False
        return _has_error_marker(text, extra_markers=("bfgs failed",))


@dataclass
class PhOut:
    path: str

    def exists(self) -> bool:
        return os.path.exists(self.path)

    def text(self) -> str:
        return _read_if_exists(self.path)

    def is_finished(self) -> bool:
        return "JOB DONE" in self.text()

    def has_error(self) -> bool:
        text = self.text()
        if not text:
            return False
        return _has_error_marker(text)


@dataclass
class Q2rOut:
    path: str

    def exists(self) -> bool:
        return os.path.exists(self.path)

    def text(self) -> str:
        return _read_if_exists(self.path)

    def is_finished(self) -> bool:
        return "JOB DONE" in self.text()

    def has_error(self) -> bool:
        text = self.text()
        if not text:
            return False
        return _has_error_marker(text)


@dataclass
class MatdynOut:
    path: str

    def exists(self) -> bool:
        return os.path.exists(self.path)

    def text(self) -> str:
        return _read_if_exists(self.path)

    def is_finished(self) -> bool:
        return "JOB DONE" in self.text()

    def has_error(self) -> bool:
        text = self.text()
        if not text:
            return False
        return _has_error_marker(text)
