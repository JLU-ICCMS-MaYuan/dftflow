"""Compatibility helpers for the vendored A2F calculation code."""

from scipy import integrate


def simpson(y, x=None):
    """Use the current SciPy API while supporting older SciPy releases."""

    if hasattr(integrate, "simpson"):
        return integrate.simpson(y, x=x)
    return integrate.simps(y, x=x)
