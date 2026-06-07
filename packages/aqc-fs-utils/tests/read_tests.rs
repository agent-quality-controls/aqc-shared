//! Behavior probes for the fixed read rules in `plan.md`.

#![expect(
    clippy::expect_used,
    reason = "Fixture helpers outside #[test] fns assert setup success; a failed expect is the test failing."
)]
#![expect(
    clippy::disallowed_methods,
    reason = "Test fixtures write real files/dirs on purpose; the crate under test is the sanctioned fs layer."
)]

mod read {
    use std::fs;

    use aqc_fs_utils::{
        ReadBytesOptions, ReadError, ReadTextOptions, SymlinkReadPolicy, read_bytes, read_text,
    };

    /// A temp dir holding one file with the given bytes.
    fn file_with(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("fixture must create a temp dir");
        let path = dir.path().join("f.txt");
        fs::write(&path, bytes).expect("fixture must write the file");
        (dir, path)
    }

    #[test]
    fn empty_file_is_ok_empty_string() {
        let (_d, p) = file_with(b"");
        let out = read_text(&p, &ReadTextOptions::default());
        assert!(matches!(out, Ok(ref s) if s.is_empty()), "{out:?}");
    }

    #[test]
    fn missing_file_is_not_found() {
        let dir = tempfile::tempdir().expect("fixture must create a temp dir");
        let out = read_text(dir.path().join("absent"), &ReadTextOptions::default());
        assert!(matches!(out, Err(ReadError::NotFound)), "{out:?}");
    }

    #[test]
    fn directory_is_not_a_file() {
        let dir = tempfile::tempdir().expect("fixture must create a temp dir");
        let out = read_text(dir.path(), &ReadTextOptions::default());
        assert!(matches!(out, Err(ReadError::NotAFile)), "{out:?}");
    }

    #[test]
    fn nul_byte_is_rejected_before_decode() {
        let (_d, p) = file_with(b"hello\0world");
        let out = read_text(&p, &ReadTextOptions::default());
        assert!(matches!(out, Err(ReadError::ContainsNulByte)), "{out:?}");
        // bytes read does NOT reject NUL
        let raw = read_bytes(&p, &ReadBytesOptions::default());
        assert!(matches!(raw, Ok(ref b) if b.len() == 11), "{raw:?}");
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        let (_d, p) = file_with(&[0xff, 0xfe, 0x41]);
        let out = read_text(&p, &ReadTextOptions::default());
        assert!(matches!(out, Err(ReadError::InvalidUtf8)), "{out:?}");
    }

    #[test]
    fn too_large_is_rejected() {
        let (_d, p) = file_with(b"0123456789");
        let options = ReadTextOptions {
            max_bytes: 9,
            ..ReadTextOptions::default()
        };
        let out = read_text(&p, &options);
        assert!(matches!(out, Err(ReadError::TooLarge)), "{out:?}");
    }

    #[test]
    fn crlf_normalization_is_opt_in() {
        let (_d, p) = file_with(b"a\r\nb\r\n");
        let plain = read_text(&p, &ReadTextOptions::default()).expect("plain read succeeds");
        assert_eq!(plain, "a\r\nb\r\n", "no normalization by default");
        let normalized = read_text(
            &p,
            &ReadTextOptions {
                normalize_crlf: true,
                ..ReadTextOptions::default()
            },
        )
        .expect("normalized read succeeds");
        assert_eq!(normalized, "a\nb\n", "CRLF collapsed when requested");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_policies() {
        let dir = tempfile::tempdir().expect("fixture must create a temp dir");
        let target = dir.path().join("target.txt");
        fs::write(&target, b"via link").expect("fixture must write the target");
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).expect("fixture must create the symlink");

        let dont = read_text(&link, &ReadTextOptions::default());
        assert!(
            matches!(dont, Err(ReadError::NotAFile)),
            "DontFollow treats the symlink node as not-a-file: {dont:?}"
        );
        let followed = read_text(
            &link,
            &ReadTextOptions {
                symlink: SymlinkReadPolicy::Follow,
                ..ReadTextOptions::default()
            },
        )
        .expect("Follow reads the target");
        assert_eq!(followed, "via link");
    }
}
