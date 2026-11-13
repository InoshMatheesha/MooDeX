// cmdline.rs - Windows command-line quoting (CommandLineToArgvW rules).

// Quote one argument so the child's argv sees it unchanged.
fn quote_arg(arg: &str, out: &mut String) {
    let needs = arg.is_empty() || arg.chars().any(|c| matches!(c, ' ' | '\t' | '\n' | '\u{0b}' | '"'));
    if !needs {
        out.push_str(arg);
        return;
    }
    out.push('"');
    let mut backslashes = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
}

// Arguments only, quoted and space-separated.
pub fn join_args(args: &[String]) -> String {
    let mut s = String::new();
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        quote_arg(a, &mut s);
    }
    s
}

// Program path (always quoted) followed by quoted arguments.
pub fn build_command_line(program: &str, args: &[String]) -> String {
    let mut s = String::with_capacity(program.len() + 2 + args.iter().map(|a| a.len() + 3).sum::<usize>());
    s.push('"');
    s.push_str(program);
    s.push('"');
    for a in args {
        s.push(' ');
        quote_arg(a, &mut s);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_rules() {
        assert_eq!(build_command_line("C:\\a b\\g.exe", &[]), "\"C:\\a b\\g.exe\"");
        let args = vec![
            "-x".to_string(),
            "two words".to_string(),
            "".to_string(),
            "say \"hi\"".to_string(),
            "C:\\dir\\".to_string(),
        ];
        assert_eq!(build_command_line("g.exe", &args), "\"g.exe\" -x \"two words\" \"\" \"say \\\"hi\\\"\" C:\\dir\\");
        assert_eq!(build_command_line("g.exe", &["a b\\".to_string()]), "\"g.exe\" \"a b\\\\\"");
        assert_eq!(join_args(&["a".to_string(), "b c".to_string()]), "a \"b c\"");
    }
}
