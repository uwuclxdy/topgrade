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

// case 7: a crate-root local type named `Executor`. Crate-local matching fires on any local
// `Executor` regardless of module location, so this FIRES (it was the negative before the match
// was made rename-robust). The decoy `Other` (case 6) is the non-`Executor` negative.
struct Executor {
    argv: Vec<String>,
}

impl Executor {
    fn arg<S: AsRef<OsStr>>(&mut self, s: S) -> &mut Self {
        self.argv.push(s.as_ref().to_string_lossy().into_owned());
        self
    }
}

// case 8: a local `Executor` nested in a module. Fires like case 7 -- crate-local matching is
// module-location-independent. Mirrors topgrade's own `Executor` in `src/executor.rs`.
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

    // case 7: crate-root local `Executor` -> MUST FIRE (crate-local match, module-independent)
    let mut local_exec = Executor { argv: Vec::new() };
    local_exec.arg("-z");

    // case 8: `executor::Executor` (module path) -> MUST FIRE on `-z`
    let mut mod_exec = executor::Executor { argv: Vec::new() };
    mod_exec.arg("-z");
}
