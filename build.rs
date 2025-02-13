// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

trait CommandExt {
    fn checked(&mut self);
}

impl CommandExt for Command {
    fn checked(&mut self) {
        let status = self.status().unwrap();
        if !status.success() {
            panic!("Command {:?} failed with status {status}", self);
        }
    }
}

pub fn compile_resources<P: AsRef<Path>>(
    source_dirs: &[P],
    gresource: &str,
    target: &str,
) -> Vec<PathBuf> {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let mut command = Command::new("glib-compile-resources");

    for source_dir in source_dirs {
        command.arg("--sourcedir").arg(source_dir.as_ref());
    }

    command
        .arg("--target")
        .arg(out_dir.join(target))
        .arg(gresource)
        .checked();

    let mut command = Command::new("glib-compile-resources");
    for source_dir in source_dirs {
        command.arg("--sourcedir").arg(source_dir.as_ref());
    }

    let output = command
        .arg("--generate-dependencies")
        .arg(gresource)
        .stderr(Stdio::inherit())
        .output()
        .unwrap()
        .stdout;

    let mut sources = vec![Path::new(gresource).into()];

    for line in String::from_utf8(output).unwrap().lines() {
        if line.ends_with(".ui") {
            // We build UI files from blueprint, so adapt the dependency
            sources.push(Path::new(line).with_extension("blp"))
        } else if line.ends_with(".metainfo.xml") {
            sources.push(Path::new(line).with_extension("xml.in"));
        } else {
            sources.push(line.into());
        }
    }

    sources
}

fn main() {
    let mut sources = Vec::new();

    sources.extend_from_slice(
        compile_resources(
            &["resources"],
            "resources/resources.gresource.xml",
            "picture-of-the-day.gresource",
        )
        .as_slice(),
    );

    for source in sources {
        println!("cargo:rerun-if-changed={}", source.display());
    }
}
