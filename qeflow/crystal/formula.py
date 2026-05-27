#!/usr/bin/env python3
from typing import Sequence


def get_formula(elements: Sequence[str], counts: Sequence[int]) -> str:
    formula = ""
    for element, count in zip(elements, counts):
        formula += str(element)
        if int(count) >= 1:
            formula += str(int(count))
    return formula

