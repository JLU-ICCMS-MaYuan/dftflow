#!/usr/bin/env python3
from typing import Dict, Iterable, List


ELEMENT_SYMBOLS = [
    None,
    "H", "He",
    "Li", "Be", "B", "C", "N", "O", "F", "Ne",
    "Na", "Mg", "Al", "Si", "P", "S", "Cl", "Ar",
    "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn",
    "Ga", "Ge", "As", "Se", "Br", "Kr",
    "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd",
    "In", "Sn", "Sb", "Te", "I", "Xe",
    "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd", "Tb", "Dy",
    "Ho", "Er", "Tm", "Yb", "Lu",
    "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg",
    "Tl", "Pb", "Bi", "Po", "At", "Rn",
    "Fr", "Ra", "Ac", "Th", "Pa", "U", "Np", "Pu", "Am", "Cm", "Bk", "Cf",
    "Es", "Fm", "Md", "No", "Lr",
    "Rf", "Db", "Sg", "Bh", "Hs", "Mt", "Ds", "Rg", "Cn", "Nh", "Fl", "Mc",
    "Lv", "Ts", "Og",
]

PERIOD_RANGES = {
    1: range(1, 3),
    2: range(3, 11),
    3: range(11, 19),
    4: range(19, 37),
    5: range(37, 55),
    6: range(55, 87),
    7: range(87, 119),
}

DISPLAY_CELL_WIDTH = 4
DISPLAY_COLUMNS = 18

PERIOD_TABLE_LAYOUT = {
    "period_1": {1: "H", 18: "He"},
    "period_2": {1: "Li", 2: "Be", 13: "B", 14: "C", 15: "N", 16: "O", 17: "F", 18: "Ne"},
    "period_3": {1: "Na", 2: "Mg", 13: "Al", 14: "Si", 15: "P", 16: "S", 17: "Cl", 18: "Ar"},
    "period_4": {
        1: "K", 2: "Ca", 3: "Sc", 4: "Ti", 5: "V", 6: "Cr", 7: "Mn", 8: "Fe", 9: "Co",
        10: "Ni", 11: "Cu", 12: "Zn", 13: "Ga", 14: "Ge", 15: "As", 16: "Se", 17: "Br", 18: "Kr",
    },
    "period_5": {
        1: "Rb", 2: "Sr", 3: "Y", 4: "Zr", 5: "Nb", 6: "Mo", 7: "Tc", 8: "Ru", 9: "Rh",
        10: "Pd", 11: "Ag", 12: "Cd", 13: "In", 14: "Sn", 15: "Sb", 16: "Te", 17: "I", 18: "Xe",
    },
    "period_6": {
        1: "Cs", 2: "Ba", 3: "La", 4: "Hf", 5: "Ta", 6: "W", 7: "Re", 8: "Os", 9: "Ir",
        10: "Pt", 11: "Au", 12: "Hg", 13: "Tl", 14: "Pb", 15: "Bi", 16: "Po", 17: "At", 18: "Rn",
    },
    "period_7": {
        1: "Fr", 2: "Ra", 3: "Ac", 4: "Rf", 5: "Db", 6: "Sg", 7: "Bh", 8: "Hs", 9: "Mt",
        10: "Ds", 11: "Rg", 12: "Cn", 13: "Nh", 14: "Fl", 15: "Mc", 16: "Lv", 17: "Ts", 18: "Og",
    },
}

SERIES_TABLE_LAYOUT = {
    "lanthanides": {
        3: "La", 4: "Ce", 5: "Pr", 6: "Nd", 7: "Pm", 8: "Sm", 9: "Eu", 10: "Gd",
        11: "Tb", 12: "Dy", 13: "Ho", 14: "Er", 15: "Tm", 16: "Yb", 17: "Lu",
    },
    "actinides": {
        3: "Ac", 4: "Th", 5: "Pa", 6: "U", 7: "Np", 8: "Pu", 9: "Am", 10: "Cm",
        11: "Bk", 12: "Cf", 13: "Es", 14: "Fm", 15: "Md", 16: "No", 17: "Lr",
    },
}


def atomic_number_to_symbol(number: int) -> str:
    if number < 1 or number >= len(ELEMENT_SYMBOLS):
        raise ValueError(f"原子序数超出范围: {number}，仅支持 1-118。")
    return ELEMENT_SYMBOLS[number]


def symbol_to_atomic_number(symbol: str) -> int:
    for index, current in enumerate(ELEMENT_SYMBOLS):
        if current == symbol:
            return index
    raise ValueError(f"未知元素符号: {symbol}")


def atomic_number_to_period(number: int) -> int:
    for period, values in PERIOD_RANGES.items():
        if number in values:
            return period
    raise ValueError(f"无法确定元素 {number} 的周期。")


def parse_atomic_selection(selection: str) -> List[int]:
    values = set()
    for item in selection.split(","):
        token = item.strip()
        if not token:
            continue
        if "-" in token:
            parts = token.split("-", 1)
            if len(parts) != 2 or not parts[0].strip().isdigit() or not parts[1].strip().isdigit():
                raise ValueError(f"非法区间: {token}")
            start = int(parts[0])
            end = int(parts[1])
            if start > end:
                raise ValueError(f"区间起点大于终点: {token}")
            for number in range(start, end + 1):
                atomic_number_to_symbol(number)
                values.add(number)
            continue
        if not token.isdigit():
            raise ValueError(f"非法原子序数字段: {token}")
        number = int(token)
        atomic_number_to_symbol(number)
        values.add(number)
    if not values:
        raise ValueError("原子序号选择不能为空。")
    return sorted(values)


def selection_to_symbols(selection: str) -> List[str]:
    return [atomic_number_to_symbol(number) for number in parse_atomic_selection(selection)]


def group_symbols_by_period(symbols: Iterable[str]) -> Dict[int, List[str]]:
    grouped: Dict[int, List[str]] = {period: [] for period in PERIOD_RANGES}
    for symbol in symbols:
        number = symbol_to_atomic_number(symbol)
        grouped[atomic_number_to_period(number)].append(symbol)
    return grouped


def render_selection_preview(label: str, selection: str) -> str:
    numbers = parse_atomic_selection(selection)
    selected = {atomic_number_to_symbol(number) for number in numbers}
    lines = [f"{label} 选择预览:"]

    def render_layout_row(row_label: str, layout: Dict[int, str]) -> str:
        cells = []
        count = 0
        for column in range(1, DISPLAY_COLUMNS + 1):
            symbol = layout.get(column)
            if symbol and symbol in selected:
                cells.append(f"{symbol:<{DISPLAY_CELL_WIDTH}}")
                count += 1
            else:
                cells.append(" " * DISPLAY_CELL_WIDTH)
        return f"({count:02d}){row_label}: {''.join(cells).rstrip()}"

    for row_label, layout in PERIOD_TABLE_LAYOUT.items():
        lines.append(render_layout_row(row_label, layout))
    for row_label, layout in SERIES_TABLE_LAYOUT.items():
        lines.append(render_layout_row(row_label, layout))
    return "\n".join(lines)
