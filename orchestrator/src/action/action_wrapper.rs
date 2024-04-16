use std::{any::TypeId, fmt::format, fs::create_dir_all, path::{Path, PathBuf}};

use super::{command_list::CommandListAction, emergency::EmergencyAction, Action};



#[derive(Debug)]
pub struct ActionWrapper {
    pub action: Option<Box<dyn Action>>, // will be None if this is a placeholder
    pub ctx: Context,
}

#[derive(Debug, Clone)]
pub struct Context {
    id: ActionId,
    type_name: String,
    save_dir: PathBuf,
}

pub type ActionId = u32;


impl ActionWrapper {
    pub fn new<A: Action + 'static>(action: A, id: ActionId) -> ActionWrapper {
        ActionWrapper {
            action: Some(Box::new(action)),
            ctx: Context {
                id,
                type_name: A::get_type_name().to_string(),
                save_dir: PathBuf::new(),
            },
        }
    }

    pub fn get_id(&self) -> ActionId {
        self.ctx.id
    }

    pub fn load_from_disk(dir: &Path) -> Result<ActionWrapper, String> {
        if !dir.is_dir() {
            return Err(format!("Not a directory: {dir:?}"))
        }
        let filename = dir.file_name()
            .ok_or_else(|| format!("Directory does not exist: {dir:?}"))?
            .to_string_lossy();

        let mut filename_pieces = filename.splitn(2, '_');
        let id = filename_pieces.next().ok_or_else(|| format!("Invalid filename: {filename}"))?;
        let id = id.parse::<ActionId>().map_err(|e| format!("Invalid filename: {filename}: {e}"))?;
        let type_name = filename_pieces.next().ok_or_else(|| format!("Invalid filename: {filename}"))?;
        let ctx = Context { id, type_name: type_name.to_string(), save_dir: dir.into() };

        if type_name == EmergencyAction::get_type_name() {
            Ok(ActionWrapper { action: Some(Box::new(EmergencyAction::load_from_disk(&ctx)?)), ctx })
        } else if type_name == CommandListAction::get_type_name() {
            Ok(ActionWrapper { action: Some(Box::new(CommandListAction::load_from_disk(&ctx)?)), ctx })
        } else {
            Err(format!("Invalid filename: {filename}: invalid type name {type_name}"))
        }
    }

    pub fn save_to_disk(&self) -> Result<(), String> {
        let action = if let Some(action) = &self.action {
            action
        } else {
            return Err("".to_string());
        };

        create_dir_all(&self.ctx.save_dir);
        action.save_to_disk(&self.ctx);

        Ok(())
    }
}

impl Context {
    pub fn get_save_dir(&self) -> &PathBuf {
        &self.save_dir
    }
}