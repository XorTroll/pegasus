use serde::{Serialize, Deserialize};
use std::fs::{File, create_dir};
use crate::{result::*, util::{convert_io_result, convert_serde_json_result, get_path_relative_to_cwd}};

const CONFIG_FILE: &str = "config.cfg";

const DEFAULT_NAND_SYSTEM_DIR: &str = "nand_system";
const DEFAULT_NAND_USER_DIR: &str = "nand_user";
const DEFAULT_SD_CARD_DIR: &str = "sd_card";

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub nand_system_path: String,
    pub nand_user_path: String,
    pub sd_card_path: String
}

impl Default for Config {
    fn default() -> Self {
        let nand_system_path = get_path_relative_to_cwd(DEFAULT_NAND_SYSTEM_DIR);
        let _ = create_dir(nand_system_path.clone());

        let nand_user_path = get_path_relative_to_cwd(DEFAULT_NAND_USER_DIR);
        let _ = create_dir(nand_user_path.clone());

        let sd_card_path = get_path_relative_to_cwd(DEFAULT_SD_CARD_DIR);
        let _ = create_dir(sd_card_path.clone());

        Self {
            nand_system_path: nand_system_path,
            nand_user_path: nand_user_path,
            sd_card_path: sd_card_path,
        }
    }
}

static mut G_CONFIG: Option<Config> = None;
static mut G_CONFIG_PATH: String = String::new();

pub fn get_config() -> &'static mut Config {
    unsafe {
        assert!(G_CONFIG.is_some());

        G_CONFIG.as_mut().unwrap()
    }
}

pub fn get_config_path() -> String {
    unsafe {
        assert!(!G_CONFIG_PATH.is_empty());

        G_CONFIG_PATH.clone()
    }
}

fn set_config(cfg: Config, path: String) {
    unsafe {
        G_CONFIG = Some(cfg);
        G_CONFIG_PATH = path;
    }
}

pub fn load_config(path: String) -> Result<()> {
    let file = convert_io_result(File::open(path.clone()))?;
    let cfg: Config = convert_serde_json_result(serde_json::from_reader(file))?;
    set_config(cfg, path);

    Ok(())
}

pub fn save_config() -> Result<()> {
    let file = convert_io_result(File::create(get_config_path()))?;
    convert_serde_json_result(serde_json::to_writer_pretty(file, get_config()))
}

pub fn initialize() -> Result<()> {
    let config_path = get_path_relative_to_cwd(CONFIG_FILE);
    match load_config(config_path.clone()) {
        Err(_) => {
            let default_cfg: Config = Default::default();
            set_config(default_cfg, config_path);
            save_config().unwrap();
        }
        _ => {}
    }

    Ok(())
}