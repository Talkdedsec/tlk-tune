use std::path::PathBuf;

/// Expands `~`, `%VAR%` and `$VAR` so a path copied from anywhere still
/// resolves. An unset variable is left as written rather than silently
/// collapsing the path to something shorter.
pub fn expand(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return PathBuf::new();
    }

    // A backslash separates folders on Windows and is an ordinary character
    // in a unix filename, so `~\Music` is only a home path on one of them.
    let separates = |rest: &str| {
        rest.is_empty() || rest.starts_with('/') || (cfg!(windows) && rest.starts_with('\\'))
    };

    let mut text = String::with_capacity(trimmed.len());
    if let Some(rest) = trimmed.strip_prefix('~') {
        match dirs::home_dir() {
            Some(home) if separates(rest) => {
                text.push_str(&home.to_string_lossy());
                text.push_str(rest);
            }
            _ => text.push_str(trimmed),
        }
    } else {
        text.push_str(trimmed);
    }

    let text = expand_percent(&text);
    let text = expand_dollar(&text);
    PathBuf::from(text)
}

fn expand_percent(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(open) = rest.find('%') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('%') else {
            break;
        };
        let name = &after[..close];
        match std::env::var(name) {
            Ok(value) => {
                out.push_str(&rest[..open]);
                out.push_str(&value);
            }
            Err(_) => out.push_str(&rest[..open + close + 2]),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

fn expand_dollar(input: &str) -> String {
    if !input.contains('$') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        let braced = chars.peek() == Some(&'{');
        if braced {
            chars.next();
        }
        let mut name = String::new();
        while let Some(&n) = chars.peek() {
            let ok = if braced {
                n != '}'
            } else {
                n.is_ascii_alphanumeric() || n == '_'
            };
            if !ok {
                break;
            }
            name.push(n);
            chars.next();
        }
        if braced {
            chars.next();
        }
        match std::env::var(&name) {
            Ok(value) => out.push_str(&value),
            Err(_) => {
                out.push('$');
                if braced {
                    out.push('{');
                }
                out.push_str(&name);
                if braced {
                    out.push('}');
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_home() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand("~/Music"), home.join("Music"));
        if cfg!(windows) {
            assert_eq!(expand("~\\Music"), home.join("Music"));
        } else {
            // Not a separator here, so it is a folder with an odd name.
            assert_eq!(expand("~\\Music"), PathBuf::from("~\\Music"));
        }
    }

    #[test]
    fn leaves_a_bare_tilde_word_alone() {
        assert_eq!(expand("~notauser/x"), PathBuf::from("~notauser/x"));
    }

    #[test]
    fn expands_windows_variables() {
        std::env::set_var("TLK_TUNE_TEST_DIR", "D:\\Sounds");
        assert_eq!(
            expand("%TLK_TUNE_TEST_DIR%\\Albums"),
            PathBuf::from("D:\\Sounds\\Albums")
        );
        assert_eq!(
            expand("$TLK_TUNE_TEST_DIR/Albums"),
            PathBuf::from("D:\\Sounds/Albums")
        );
    }

    #[test]
    fn keeps_unknown_variables_visible() {
        assert_eq!(
            expand("%NO_SUCH_VAR_HERE%\\x"),
            PathBuf::from("%NO_SUCH_VAR_HERE%\\x")
        );
    }
}
