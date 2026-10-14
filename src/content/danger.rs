//! Spotting commands that change or delete data, so every surface can warn
//! before showing them.

/// True when `cmd` changes or deletes data. A trailing `# comment` is ignored.
pub fn is_dangerous(cmd: &str) -> bool {
    let words: Vec<&str> = code_part(cmd).split_whitespace().collect();
    words.iter().enumerate().any(|(i, &w)| match w {
        "rm" | "dd" | "shred" | "wipefs" | "truncate" | "-delete" => true,
        "chmod" | "chown" | "chgrp" => words[i + 1..].contains(&"-R"),
        _ => w.starts_with("mkfs"),
    })
}

/// The command without its shell comment.
fn code_part(cmd: &str) -> &str {
    if cmd.trim_start().starts_with('#') {
        return "";
    }
    match cmd.find(" #") {
        Some(i) => &cmd[..i],
        None => cmd,
    }
}

#[cfg(test)]
mod tests {
    use super::is_dangerous;

    #[test]
    fn flags_destructive_commands() {
        for cmd in [
            "find . -name '*.tmp' -delete",
            "sudo rm -rf build",
            "find . -exec rm {} +",
            "dd if=disk.img of=/dev/sdb bs=4M",
            "sudo mkfs.ext4 /dev/sdb1",
            "chmod -R 755 .",
            "sudo chown alice -R /srv",
            "shred -u secret.txt",
        ] {
            assert!(is_dangerous(cmd), "{cmd}");
        }
    }

    #[test]
    fn leaves_safe_commands_alone() {
        for cmd in [
            "find . -name '*.log'",
            "chmod 644 notes.txt",
            "free -h",
            "ls -l /dev/sdb   # rm would be bad here",
            "# rm -rf /",
            "grep -r 'rm -rf' .",
        ] {
            assert!(!is_dangerous(cmd), "{cmd}");
        }
    }
}
