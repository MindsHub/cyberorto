use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};

use super::{command_list::CommandListAction, emergency::EmergencyAction, Action};

/// Holds an `Action` along with some fixed stats needed to keep track of
/// its execution and/or save it to disk. Can act as a placeholder in the
/// queue, so if the `Action` is being executed, the `ActionWrapper` in the
/// queue will have [`action`](ActionWrapper::action)`= None`.
#[derive(Debug)]
pub struct ActionWrapper {
    /// The action being wrapped.
    /// Will be None if this ActionWrapper is a placeholder.
    pub action: Option<Box<dyn Action>>,

    /// Additional information to identify the action and save it to disk.
    pub ctx: Context,
}

/// Holds information to identify an action and load/save it from/to disk.
#[derive(Debug, Clone)]
pub struct Context {
    /// A **unique** identifier for the action.
    /// Will be distinct for each action instance.
    id: ActionId,

    /// The name of the type of the action. Obtained from a call to
    /// [`Action::get_type_name()`](Action::get_type_name()) on an
    /// `Action` implementor.
    type_name: String,

    /// The directory in which to store files about this action, both during
    /// execution (e.g. to cache images/data for later usage) or because the
    /// action is being saved to disk.
    save_dir: PathBuf,
}

pub type ActionId = u32;

impl ActionWrapper {
    /// Creates a non-placeholder `ActionWrapper`, initializing the context with
    /// `A::get_type_name()`.
    ///
    /// * `action` the action to wrap (although in the end a `dyn Action` is
    ///            stored in the wrapper, its type is needed at compile time to
    ///            call the correct
    ///            [`Action::get_type_name()`](Action::get_type_name()))
    /// * `id` a **unique** ID for this action
    pub fn new<A: Action + 'static>(action: A, id: ActionId, save_dir: &Path) -> ActionWrapper {
        ActionWrapper {
            action: Some(Box::new(action)),
            ctx: Context {
                id,
                type_name: A::get_type_name().to_string(),
                save_dir: save_dir.join(format!("{}_{}", id, A::get_type_name())),
            },
        }
    }

    /// Returns the **unique** ID of the action being wrapped.
    pub fn get_id(&self) -> ActionId {
        self.ctx.id
    }

    /// Returns `A::get_type_name()` of the action type this was originally
    /// initialized from.
    pub fn get_type_name(&self) -> &String {
        &self.ctx.type_name
    }

    /// Returns the directory in which to store files about this action, both
    /// during execution (e.g. to cache images/data for later usage) or when
    /// saving the action to disk.
    pub fn get_save_dir(&self) -> &PathBuf {
        &self.ctx.save_dir
    }

    /// Returns whether this actually contains an action, or if it's just a
    /// placeholder.
    pub fn is_placeholder(&self) -> bool {
        self.action.is_none()
    }

    /// Returns an `Action` after loading it from disk at the location `dir`,
    /// or an error if something went wrong. Chooses the type of the action
    /// to load based on the TYPENAME stored in `dir`'s name.
    ///
    /// * `dir` is where to look for the files for the `Action` to load. The
    ///         name must be of the form `ID_TYPENAME`, and ID and TYPENAME
    ///         will be extracted from there.
    pub fn load_from_disk(dir: &Path) -> Result<ActionWrapper, String> {
        if !dir.is_dir() {
            return Err(format!("Not a directory: {dir:?}"));
        }
        let filename = dir
            .file_name()
            .ok_or_else(|| format!("Directory does not exist: {dir:?}"))?
            .to_string_lossy();

        let mut filename_pieces = filename.splitn(2, '_');
        let id = filename_pieces
            .next()
            .ok_or_else(|| format!("Invalid filename: {filename}"))?;
        let id = id
            .parse::<ActionId>()
            .map_err(|e| format!("Invalid filename: {filename}: {e}"))?;
        let type_name = filename_pieces
            .next()
            .ok_or_else(|| format!("Invalid filename: {filename}"))?;
        let ctx = Context {
            id,
            type_name: type_name.to_string(),
            save_dir: dir.into(),
        };

        if type_name == EmergencyAction::get_type_name() {
            Ok(ActionWrapper {
                action: Some(Box::new(EmergencyAction::load_from_disk(&ctx)?)),
                ctx,
            })
        } else if type_name == CommandListAction::get_type_name() {
            Ok(ActionWrapper {
                action: Some(Box::new(CommandListAction::load_from_disk(&ctx)?)),
                ctx,
            })
        } else {
            Err(format!(
                "Invalid filename: {filename}: invalid type name {type_name}"
            ))
        }
    }

    /// Saves this `Action` to disk, by storing files inside `self.ctx.save_dir`.
    /// Returns an error if something went wrong.
    pub fn save_to_disk(&self) -> Result<(), String> {
        let action = if let Some(action) = &self.action {
            action
        } else {
            return Err("".to_string());
        };

        create_dir_all(&self.ctx.save_dir).map_err(|e| e.to_string())?;
        action.save_to_disk(&self.ctx)?;

        Ok(())
    }

    /// Deletes all data stored by this action inside `self.ctx.save_dir`,
    /// ignoring any error.
    pub fn delete_data_on_disk(&self) {
        let _ = remove_dir_all(&self.ctx.save_dir);
    }
}

impl Context {
    /// Returns the directory in which to store files about this action, both
    /// during execution (e.g. to cache images/data for later usage) or when
    /// saving the action to disk.
    pub fn get_save_dir(&self) -> &PathBuf {
        &self.save_dir
    }
}
