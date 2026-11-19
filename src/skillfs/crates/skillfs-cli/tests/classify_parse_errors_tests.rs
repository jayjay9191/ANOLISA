use skillfs_core::views::ViewsConfig;
use std::fs;
use std::process::Command;

#[test]
fn classify_keeps_parse_errors_out_of_generated_default_view() {
    for (include_valid, count) in [(true, 2), (false, 1), (true, 0)] {
        let source = tempfile::tempdir().expect("source tempdir");
        let mut fixtures = vec![
            ("a-error", "---\nname: [\n---\n"),
            ("d-error", "---\nname: [\n---\n"),
        ];
        if include_valid {
            fixtures.extend([
                ("b-degraded", "---\nname: b-degraded\n---\n"),
                (
                    "c-valid",
                    "---\nname: c-valid\ndescription: Valid skill\n---\n",
                ),
            ]);
        }
        for (name, manifest) in fixtures {
            let skill = source.path().join(name);
            fs::create_dir(&skill).expect("create fixture directory");
            fs::write(skill.join("SKILL.md"), manifest).expect("write fixture");
        }

        let views_path = source.path().join("skillfs-views.toml");
        let mut listings = Vec::new();
        for dry_run in [true, false] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_skillfs"));
            command
                .arg("classify")
                .arg(source.path())
                .args(["--primary-count", &count.to_string()])
                .env("SKILLFS_SLS_OPS_PATH", source.path().join("ops.jsonl"));
            if dry_run {
                command.arg("--dry-run");
            }
            let output = command.output().expect("run classify");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{stderr}");
            for name in ["a-error", "d-error"] {
                assert!(
                    stderr.lines().any(|line| line.contains(name)
                        && line.contains("parse error")
                        && line.contains("secondary view")),
                    "the warning must name {name} and explain its assignment: {stderr}"
                );
            }
            assert_eq!(views_path.exists(), !dry_run);

            let stdout = String::from_utf8(output.stdout).expect("UTF-8 classify output");
            let (_, listing) = stdout.split_once("\n\n").expect("view listing");
            let listing = listing
                .split_once("\n\nEdit ")
                .map_or(listing, |(views, _)| views);
            listings.push(listing.trim_end().to_owned());
        }
        assert_eq!(listings[0], listings[1]);

        let views = ViewsConfig::load(source.path()).expect("read generated views");
        let (major, other): (&[&str], &[&str]) = match (include_valid, count) {
            (true, 2) => (&["b-degraded", "c-valid"], &["a-error", "d-error"]),
            (true, 0) => (&[], &["a-error", "b-degraded", "c-valid", "d-error"]),
            _ => (&[], &["a-error", "d-error"]),
        };
        assert_eq!(views.default_skills(), major);
        assert_eq!(views.views[1].skills, other);
    }
}
