use std::{fs, path::Path};

use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct TideConfig {
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub lsp: LspConfig,
}

#[derive(Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_explorer_width")]
    pub explorer_width: u16,
    #[serde(default)]
    pub show_hidden: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            explorer_width: default_explorer_width(),
            show_hidden: false,
        }
    }
}

#[derive(Default, Deserialize)]
pub struct LspConfig {
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub disabled: bool,
}

fn default_explorer_width() -> u16 {
    28
}

impl TideConfig {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(".tide.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut config: Self =
            toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        config.editor.explorer_width = config.editor.explorer_width.clamp(15, 60);
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_clamps_project_configuration() {
        let root = std::env::temp_dir().join(format!("tide-config-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join(".tide.toml"),
            "[editor]\nexplorer_width = 99\nshow_hidden = true\n[lsp]\ncommand = [\"example-lsp\", \"--stdio\"]\n",
        )
        .unwrap();
        let config = TideConfig::load(&root).unwrap();
        assert_eq!(config.editor.explorer_width, 60);
        assert!(config.editor.show_hidden);
        assert_eq!(config.lsp.command, ["example-lsp", "--stdio"]);
        fs::remove_dir_all(root).unwrap();
    }
}
