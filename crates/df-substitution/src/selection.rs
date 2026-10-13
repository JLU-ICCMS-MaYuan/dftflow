//! 元素符号表与选择解析（复刻 `substitutionflow/selection.py`）。

use anyhow::{bail, Result};

pub const ELEMENT_SYMBOLS: [&str; 119] = [
    "", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S",
    "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga", "Ge",
    "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd",
    "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd",
    "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg",
    "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa", "U", "Np", "Pu", "Am", "Cm",
    "Bk", "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db", "Sg", "Bh", "Hs", "Mt", "Ds", "Rg", "Cn",
    "Nh", "Fl", "Mc", "Lv", "Ts", "Og",
];

pub fn period_range(period: usize) -> (usize, usize) {
    match period {
        1 => (1, 3),
        2 => (3, 11),
        3 => (11, 19),
        4 => (19, 37),
        5 => (37, 55),
        6 => (55, 87),
        7 => (87, 119),
        _ => (0, 0),
    }
}

pub const DISPLAY_CELL_WIDTH: usize = 4;
pub const DISPLAY_COLUMNS: usize = 18;

/// (row_label, column) → 元素符号。
pub const PERIOD_TABLE_LAYOUT: &[(&str, &[(usize, &str)])] = &[
    ("period_1", &[(1, "H"), (18, "He")]),
    (
        "period_2",
        &[
            (1, "Li"), (2, "Be"), (13, "B"), (14, "C"), (15, "N"), (16, "O"), (17, "F"), (18, "Ne"),
        ],
    ),
    (
        "period_3",
        &[
            (1, "Na"), (2, "Mg"), (13, "Al"), (14, "Si"), (15, "P"), (16, "S"), (17, "Cl"), (18, "Ar"),
        ],
    ),
    (
        "period_4",
        &[
            (1, "K"), (2, "Ca"), (3, "Sc"), (4, "Ti"), (5, "V"), (6, "Cr"), (7, "Mn"), (8, "Fe"),
            (9, "Co"), (10, "Ni"), (11, "Cu"), (12, "Zn"), (13, "Ga"), (14, "Ge"), (15, "As"),
            (16, "Se"), (17, "Br"), (18, "Kr"),
        ],
    ),
    (
        "period_5",
        &[
            (1, "Rb"), (2, "Sr"), (3, "Y"), (4, "Zr"), (5, "Nb"), (6, "Mo"), (7, "Tc"), (8, "Ru"),
            (9, "Rh"), (10, "Pd"), (11, "Ag"), (12, "Cd"), (13, "In"), (14, "Sn"), (15, "Sb"),
            (16, "Te"), (17, "I"), (18, "Xe"),
        ],
    ),
    (
        "period_6",
        &[
            (1, "Cs"), (2, "Ba"), (3, "La"), (4, "Hf"), (5, "Ta"), (6, "W"), (7, "Re"), (8, "Os"),
            (9, "Ir"), (10, "Pt"), (11, "Au"), (12, "Hg"), (13, "Tl"), (14, "Pb"), (15, "Bi"),
            (16, "Po"), (17, "At"), (18, "Rn"),
        ],
    ),
    (
        "period_7",
        &[
            (1, "Fr"), (2, "Ra"), (3, "Ac"), (4, "Rf"), (5, "Db"), (6, "Sg"), (7, "Bh"), (8, "Hs"),
            (9, "Mt"), (10, "Ds"), (11, "Rg"), (12, "Cn"), (13, "Nh"), (14, "Fl"), (15, "Mc"),
            (16, "Lv"), (17, "Ts"), (18, "Og"),
        ],
    ),
];

pub const SERIES_TABLE_LAYOUT: &[(&str, &[(usize, &str)])] = &[
    (
        "lanthanides",
        &[
            (3, "La"), (4, "Ce"), (5, "Pr"), (6, "Nd"), (7, "Pm"), (8, "Sm"), (9, "Eu"), (10, "Gd"),
            (11, "Tb"), (12, "Dy"), (13, "Ho"), (14, "Er"), (15, "Tm"), (16, "Yb"), (17, "Lu"),
        ],
    ),
    (
        "actinides",
        &[
            (3, "Ac"), (4, "Th"), (5, "Pa"), (6, "U"), (7, "Np"), (8, "Pu"), (9, "Am"), (10, "Cm"),
            (11, "Bk"), (12, "Cf"), (13, "Es"), (14, "Fm"), (15, "Md"), (16, "No"), (17, "Lr"),
        ],
    ),
];

pub fn atomic_number_to_symbol(number: usize) -> Result<&'static str> {
    if number < 1 || number >= ELEMENT_SYMBOLS.len() {
        bail!("原子序数超出范围: {number}，仅支持 1-118。");
    }
    Ok(ELEMENT_SYMBOLS[number])
}

pub fn symbol_to_atomic_number(symbol: &str) -> Result<usize> {
    for (index, current) in ELEMENT_SYMBOLS.iter().enumerate().skip(1) {
        if *current == symbol {
            return Ok(index);
        }
    }
    bail!("未知元素符号: {symbol}")
}

pub fn atomic_number_to_period(number: usize) -> Result<usize> {
    for period in 1..=7 {
        let (lo, hi) = period_range(period);
        if number >= lo && number < hi {
            return Ok(period);
        }
    }
    bail!("无法确定元素 {number} 的周期。")
}

pub fn parse_atomic_selection(selection: &str) -> Result<Vec<usize>> {
    let mut values: Vec<usize> = Vec::new();
    for item in selection.split(',') {
        let token = item.trim();
        if token.is_empty() {
            continue;
        }
        if token.contains('-') {
            let parts: Vec<&str> = token.splitn(2, '-').collect();
            if parts.len() != 2
                || parts[0].trim().is_empty()
                || parts[1].trim().is_empty()
                || !parts[0].trim().chars().all(|c| c.is_ascii_digit())
                || !parts[1].trim().chars().all(|c| c.is_ascii_digit())
            {
                bail!("非法区间: {token}");
            }
            let start: usize = parts[0].trim().parse()?;
            let end: usize = parts[1].trim().parse()?;
            if start > end {
                bail!("区间起点大于终点: {token}");
            }
            for number in start..=end {
                atomic_number_to_symbol(number)?;
                if !values.contains(&number) {
                    values.push(number);
                }
            }
            continue;
        }
        if !token.chars().all(|c| c.is_ascii_digit()) {
            bail!("非法原子序数字段: {token}");
        }
        let number: usize = token.parse()?;
        atomic_number_to_symbol(number)?;
        if !values.contains(&number) {
            values.push(number);
        }
    }
    if values.is_empty() {
        bail!("原子序号选择不能为空。");
    }
    values.sort_unstable();
    Ok(values)
}

pub fn selection_to_symbols(selection: &str) -> Result<Vec<String>> {
    parse_atomic_selection(selection)?
        .into_iter()
        .map(|n| atomic_number_to_symbol(n).map(|s| s.to_string()))
        .collect()
}

pub fn group_symbols_by_period(symbols: &[String]) -> Result<Vec<(usize, Vec<String>)>> {
    let mut grouped: Vec<(usize, Vec<String>)> = (1..=7).map(|p| (p, Vec::new())).collect();
    for symbol in symbols {
        let number = symbol_to_atomic_number(symbol)?;
        let period = atomic_number_to_period(number)?;
        grouped[period - 1].1.push(symbol.clone());
    }
    Ok(grouped)
}

fn render_layout_row(row_label: &str, layout: &[(usize, &str)], selected: &[&str]) -> String {
    let mut cells = Vec::new();
    let mut count = 0usize;
    for column in 1..=DISPLAY_COLUMNS {
        let symbol = layout.iter().find(|(c, _)| *c == column).map(|(_, s)| *s);
        match symbol {
            Some(s) if selected.contains(&s) => {
                cells.push(format!("{s:<DISPLAY_CELL_WIDTH$}"));
                count += 1;
            }
            _ => cells.push(" ".repeat(DISPLAY_CELL_WIDTH)),
        }
    }
    let padding = " ".repeat(row_label.len().max("lanthanides".len()) - row_label.len());
    format!("({count:02}){row_label}:{padding} {}", cells.join("").trim_end())
}

/// `render_selection_preview`。
pub fn render_selection_preview(label: &str, selection: &str) -> Result<String> {
    let numbers = parse_atomic_selection(selection)?;
    let selected: Vec<&str> = numbers
        .iter()
        .filter_map(|n| ELEMENT_SYMBOLS.get(*n).copied())
        .filter(|s| !s.is_empty())
        .collect();
    let mut lines = vec![format!("{label} 选择预览:")];
    for (row_label, layout) in PERIOD_TABLE_LAYOUT {
        lines.push(render_layout_row(row_label, layout, &selected));
    }
    for (row_label, layout) in SERIES_TABLE_LAYOUT {
        lines.push(render_layout_row(row_label, layout, &selected));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_selection_ranges() {
        assert_eq!(parse_atomic_selection("3,5-8").unwrap(), vec![3, 5, 6, 7, 8]);
        assert!(parse_atomic_selection("118").is_ok());
        assert!(parse_atomic_selection("119").is_err());
        assert!(parse_atomic_selection("5-3").is_err());
    }

    #[test]
    fn preview_contains_selected_symbols() {
        let text = render_selection_preview("XA", "3,11").unwrap();
        assert!(text.contains("Li"));
        assert!(text.contains("Na"));
        assert!(text.contains("(01)"));
    }
}
