#![allow(dead_code, unused)]

use std::ffi::OsStr;
use std::process::Command;

// case 6 decoy: has its own `arg`, but is NOT a command builder.
struct Other;

impl Other {
    fn arg(&mut self, _s: &str) -> &mut Self {
        self
    }
}

// case 7 negative: a LOCAL type named `Executor`. The lint matches the qualified
// path `executor::Executor`; this crate-root local type prints as `Executor`, so it
// must NOT fire. Proves the match is path-based, not a bare-name match.
struct Executor {
    argv: Vec<String>,
}

impl Executor {
    fn arg<S: AsRef<OsStr>>(&mut self, s: S) -> &mut Self {
        self.argv.push(s.as_ref().to_string_lossy().into_owned());
        self
    }
}

// case 8 positive: a type at the qualified path `executor::Executor`. `def_path_str`
// drops the crate-under-lint's name, so this resolves to exactly `executor::Executor` --
// identical to topgrade's own `src/executor.rs` (module `executor`), the arm that fires
// 131x on real topgrade. This gives the real firing arm in-suite coverage.
mod executor {
    use std::ffi::OsStr;

    pub struct Executor {
        pub argv: Vec<String>,
    }

    impl Executor {
        pub fn arg<S: AsRef<OsStr>>(&mut self, s: S) -> &mut Self {
            self.argv.push(s.as_ref().to_string_lossy().into_owned());
            self
        }
    }
}

fn main() {
    // case 1: Command builder chain -> MUST FIRE on `-Syu`
    Command::new("pacman").arg("-Syu");

    // case 2: SPLIT BINDING -> MUST FIRE on `-i` across the statement boundary
    let mut cmd = Command::new("bash");
    let _x = 1;
    cmd.arg("-i");

    // case 3: long flag -> must NOT fire
    Command::new("apt").arg("--yes");

    // case 4: allowlisted -> must NOT fire
    Command::new("apt").arg("-y");

    // case 5: args array -> MUST FIRE on `-f` only
    Command::new("docker").args(["image", "prune", "-f"]);

    // case 6: DECOY -> must NOT fire (type scoping beats grep)
    let mut other = Other;
    other.arg("-x");

    // case 7: crate-root LOCAL `Executor` -> must NOT fire (path-based, not name match)
    let mut local_exec = Executor { argv: Vec::new() };
    local_exec.arg("-z");

    // case 8: `executor::Executor` (module path) -> MUST FIRE on `-z`
    let mut mod_exec = executor::Executor { argv: Vec::new() };
    mod_exec.arg("-z");
}
