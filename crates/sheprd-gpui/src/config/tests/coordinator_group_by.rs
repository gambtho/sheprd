use super::*;
use crate::config::preferences::Preference;

#[test]
fn coordinator_group_by_defaults_to_device_and_loads_only_known_names() -> anyhow::Result<()> {
    assert_eq!(
        Config::default().coordinator_group_by,
        CoordinatorGroupBy::Device
    );
    for text in [
        "",
        DEFAULT_CONFIG,
        "layout = 'coordinator'",
        "[layout]\nmode = 'coordinator'",
    ] {
        assert_eq!(
            Config::parse(text)?.coordinator_group_by,
            CoordinatorGroupBy::Device
        );
    }
    for (name, expected) in [
        ("device", CoordinatorGroupBy::Device),
        ("project", CoordinatorGroupBy::Project),
    ] {
        let config = Config::parse(&format!("coordinator_group_by = '{name}'"))?;
        assert_eq!(config.coordinator_group_by, expected);
        assert!(config.unknown_keys.is_empty());
    }
    for value in ["'host'", "'Project'", "true", "1"] {
        assert!(Config::parse(&format!("coordinator_group_by = {value}")).is_err());
    }
    Ok(())
}

#[test]
fn coordinator_group_by_saves_without_changing_layout_or_other_preferences() -> anyhow::Result<()> {
    let temp = TempDirectory::new()?;
    let path = temp.0.join("local.toml");
    fs::write(
        &path,
        "# personal\nlayout = 'coordinator' # kept\n[sidebar]\ngap = 3\n",
    )?;
    for group_by in [CoordinatorGroupBy::Project, CoordinatorGroupBy::Device] {
        Config::save_preference_path(Preference::CoordinatorGroupBy(group_by), &path)?;
        let text = fs::read_to_string(&path)?;
        assert!(text.contains("# personal") && text.contains("# kept"));
        let config = Config::parse(&text)?;
        assert_eq!(config.coordinator_group_by, group_by);
        assert_eq!(config.layout.mode, LayoutMode::Coordinator);
        assert!(text.contains("gap = 3"));
    }
    Ok(())
}

#[test]
fn coordinator_group_by_uses_validated_raw_config_edits() -> anyhow::Result<()> {
    let temp = TempDirectory::new()?;
    let managed = temp.0.join("config-gpui.toml");
    let local = managed.with_extension("local.toml");
    let daemon = temp.0.join("absent.toml");
    fs::write(&local, "layout = 'minimal'\n")?;
    Config::edit_key_at(
        &local,
        &daemon,
        "coordinator_group_by",
        Some(parse_value("project")),
    )?;
    assert_eq!(
        Config::check_files_at(&managed, &daemon)?.coordinator_group_by,
        CoordinatorGroupBy::Project
    );
    let before = fs::read_to_string(&local)?;
    assert!(
        Config::edit_key_at(
            &local,
            &daemon,
            "coordinator_group_by",
            Some(parse_value("host"))
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(&local)?, before);
    Config::edit_key_at(&local, &daemon, "coordinator_group_by", None)?;
    assert_eq!(
        Config::check_files_at(&managed, &daemon)?.coordinator_group_by,
        CoordinatorGroupBy::Device
    );
    Ok(())
}
