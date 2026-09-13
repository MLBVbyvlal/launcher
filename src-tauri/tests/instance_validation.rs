//! Black-box tests for instance-name validation. Every Tauri command that
//! takes an `instance_name` funnels it through this check before the name
//! reaches `join()` or `remove_dir_all`, so the adversarial cases here are
//! the ones that matter.
use tauri_app_lib::launcher::valid_instance_name;

#[test]
fn accepts_ordinary_names() {
    for name in [
        "Steve",
        "a",
        "my instance 1",
        "LB-nextgen_1.21+test",
        "a.b",
        "1",
    ] {
        assert!(valid_instance_name(name).is_ok(), "{name:?} should be accepted");
    }
    assert!(valid_instance_name(&"x".repeat(64)).is_ok());
}

#[test]
fn rejects_path_escapes() {
    for name in [
        "",
        "../evil",
        "..\\evil",
        "a/b",
        "a\\b",
        "a:b",
        "a\0b",
        ".",
        "..",
        ".hidden",
        "trailing.",
        "trailing ",
    ] {
        assert!(valid_instance_name(name).is_err(), "{name:?} should be rejected");
    }
    assert!(valid_instance_name(&"x".repeat(65)).is_err());
}

#[test]
fn rejects_windows_device_names() {
    for name in [
        "CON", "con", "Con", "PRN", "AUX", "NUL", "COM1", "com9", "LPT1", "lpt4",
        // Windows reserves device names with any extension too.
        "CON.txt", "nul.dat", "com1.save",
    ] {
        assert!(valid_instance_name(name).is_err(), "{name:?} should be rejected");
    }
    // ...but a device name after the first dot is a harmless stem.
    assert!(valid_instance_name("my.com1").is_ok());
}

#[test]
fn rejects_non_ascii_names() {
    for name in ["инстанс", "minecraft_😀", "naïve"] {
        assert!(valid_instance_name(name).is_err(), "{name:?} should be rejected");
    }
}
