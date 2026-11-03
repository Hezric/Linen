use linen_tray::{portable_data_directory, settings_path};
use std::{fs, io};

#[test]
fn portable_marker_keeps_settings_and_webview_data_beside_the_running_executable() {
    let root = std::env::temp_dir().join(format!("linen-storage-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let result = (|| -> io::Result<()> {
        let installed = root.join("portable 中文");
        fs::create_dir_all(installed.join("runtime/data"))?;
        let executable = installed.join("runtime/app.exe");
        let roaming = root.join("roaming");
        for app in ["Earmuff", "Homing"] {
            assert_eq!(
                settings_path(&executable, Some(roaming.as_os_str()), app)?,
                roaming.join(app).join(format!("{app}.toml"))
            );
        }
        assert_eq!(
            portable_data_directory(&executable)?,
            None,
            "a data directory alone does not enable portable mode"
        );
        assert!(settings_path(&executable, None, "Earmuff").is_err());
        fs::write(installed.join("runtime/portable"), b"")?;
        let data = installed.join("runtime/data");
        assert_eq!(portable_data_directory(&executable)?, Some(data.clone()));
        for app in ["Earmuff", "Homing"] {
            assert_eq!(
                settings_path(&executable, None, app)?,
                data.join(format!("{app}.toml"))
            );
            assert_eq!(
                settings_path(&executable, Some(roaming.as_os_str()), app)?,
                data.join(format!("{app}.toml"))
            );
        }
        assert_eq!(
            portable_data_directory(&installed.join("app.exe"))?,
            None,
            "the marker belongs to the running executable, after runtime relaunch"
        );
        fs::remove_file(installed.join("runtime/portable"))?;
        assert_eq!(
            settings_path(&executable, Some(roaming.as_os_str()), "Earmuff")?,
            roaming.join("Earmuff/Earmuff.toml")
        );
        fs::create_dir(installed.join("runtime/portable"))?;
        assert!(
            portable_data_directory(&executable).is_err(),
            "a malformed marker must not silently select other settings"
        );
        Ok(())
    })();
    fs::remove_dir_all(root).unwrap();
    result.unwrap();
}
