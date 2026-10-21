//! 仅处理科学调用的命令前缀；其余参数、变量和重定向逐字保留。
use std::path::Path;

pub fn is_mpirun(value: &str) -> bool {
    Path::new(value)
        .file_name()
        .is_some_and(|name| name == "mpirun")
}

/// 作业中的 mpirun 由已加载环境的 PATH 决定，不绑定登录节点文件。
pub fn plain_scientific_argv(command: &[String]) -> Vec<String> {
    let start = usize::from(command.len() > 1 && command[0] == "exec");
    let mut argv = command[start..].to_vec();
    if let Some(program) = argv.first_mut() {
        if is_mpirun(program) {
            *program = "mpirun".into();
        }
    }
    argv
}

// 只定位一个 shell 词的字节范围，用现有 shell-words 去引号；不求值变量。
fn first_word(text: &str, offset: usize) -> Option<(usize, usize, String)> {
    let bytes = text.as_bytes();
    let start = offset
        + bytes
            .get(offset..)?
            .iter()
            .take_while(|b| b.is_ascii_whitespace())
            .count();
    let mut end = start;
    let mut quote = None;
    while let Some(&byte) = bytes.get(end) {
        match (quote, byte) {
            (Some(b'\''), b'\'') | (Some(b'"'), b'"') => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (Some(b'\''), _) => {}
            (_, b'\\') => {
                end += 1;
                bytes.get(end)?;
            }
            (None, b';' | b'|' | b'&' | b'<' | b'>' | b'(' | b')') => break,
            (None, b) if b.is_ascii_whitespace() => break,
            _ => {}
        }
        end += 1;
    }
    if start == end {
        return None;
    }
    let words = shell_words::split(&text[start..end]).ok()?;
    (words.len() == 1).then(|| (start, end, words[0].clone()))
}

pub fn plain_scientific_command(command: &str) -> String {
    let Some((start, mut end, mut program)) = first_word(command, 0) else {
        return command.into();
    };
    let mut program_start = start;
    let exec = program == "exec";
    if exec {
        let Some((next, next_end, next_program)) = first_word(command, end) else {
            return command.into();
        };
        // exec 的选项具有额外语义，不能当成普通科学程序移除。
        if next_program.starts_with('-') {
            return command.into();
        }
        program_start = next;
        end = next_end;
        program = next_program;
    }
    if is_mpirun(&program) {
        format!("{}mpirun{}", &command[..start], &command[end..])
    } else if exec {
        format!("{}{}", &command[..start], &command[program_start..])
    } else {
        command.into()
    }
}
