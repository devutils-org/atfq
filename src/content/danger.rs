//! Spotting commands that change or delete data, so every surface can warn
//! before showing them.

/// True when `cmd` changes or deletes data. A trailing `# comment` is ignored.
pub fn is_dangerous(cmd: &str) -> bool {
    let words: Vec<&str> = code_part(cmd).split_whitespace().collect();
    words.iter().enumerate().any(|(i, &w)| match w {
        "rm" | "dd" | "shred" | "wipefs" | "truncate" | "-delete" => true,
        "git" => git_destroys(&words[i + 1..]),
        // rsync --delete, --delete-after…: removes files from the destination.
        _ if w.starts_with("--delete") => true,
        "chmod" | "chown" | "chgrp" => words[i + 1..].contains(&"-R"),
        _ => w.starts_with("mkfs"),
    })
}

/// Git commands that throw away work: uncommitted changes, untracked
/// files, branches, stashes, or history on a remote.
fn git_destroys(args: &[&str]) -> bool {
    // Skip global options such as `-C dir` to reach the subcommand.
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        i += if matches!(args[i], "-C" | "-c") { 2 } else { 1 };
    }
    let Some(&sub) = args.get(i) else {
        return false;
    };
    let rest = &args[i + 1..];
    let has = |o: &str| rest.contains(&o);
    // `-fd`, `-xf`: short options bundled together.
    let short = |c: char| {
        rest.iter()
            .any(|a| a.starts_with('-') && !a.starts_with("--") && a.contains(c))
    };
    match sub {
        "reset" => has("--hard"),
        "clean" => short('f') || has("--force"),
        "push" => {
            short('f')
                || rest
                    .iter()
                    .any(|a| a.starts_with("--force") || a.starts_with('+'))
        }
        "branch" => short('D') || (short('d') && has("--force")),
        "stash" => matches!(rest.first(), Some(&"drop" | &"clear")),
        "checkout" => has("--") || has("."),
        "restore" => !has("--staged") || has("--worktree") || short('W'),
        "filter-branch" | "filter-repo" => true,
        "reflog" => rest.first() == Some(&"expire"),
        _ => false,
    }
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
            "rsync -a --delete src/ backup/",
            "rsync -a --delete-after src/ backup/",
            "git reset --hard origin/main",
            "git -C repo reset --hard",
            "git clean -fd",
            "git push --force",
            "git push -f origin main",
            "git push --force-with-lease",
            "git push origin +main",
            "git branch -D old-feature",
            "git stash drop",
            "git checkout -- notes.txt",
            "git restore notes.txt",
            "git restore --staged --worktree notes.txt",
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
            "git reset HEAD~1",
            "git reset --soft HEAD~1",
            "git clean -n",
            "git push origin main",
            "git branch -d merged-feature",
            "git stash list",
            "git checkout main",
            "git restore --staged notes.txt",
            "git log --oneline",
        ] {
            assert!(!is_dangerous(cmd), "{cmd}");
        }
    }
}
